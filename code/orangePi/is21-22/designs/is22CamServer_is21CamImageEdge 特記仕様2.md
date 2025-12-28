
# A. is22がプロキシURLを発行する設計

## A-1. 目的

* DBには `drive_file_id` を保存しておき、UIやLLMが「画像見たい」となったときに
  **is22が短寿命の閲覧URL（プロキシURL）**を発行する
* Driveの直リンクをそのまま出すより

  * 権限/共有設定に依存しにくい
  * URL漏えい時の被害を抑えられる（TTL・署名・アクセス制限）
  * ローカル運用（VPN/LAN）に寄せやすい

## A-2. 推奨インターフェース（is22側）

### 1) 署名付きURL発行

* `POST /api/media/link`

  * request: `{ "frame_uuid": "...", "kind": "full", "ttl_sec": 900 }`
  * response: `{ "url": "http://is22/.../media/frame/<uuid>?exp=...&sig=...", "expires_at": "..." }`

### 2) 実体配信（プロキシ）

* `GET /media/frame/{frame_uuid}?exp=...&sig=...`

  * is22が署名検証→Driveから該当ファイルを取得→JPEGをstream返却
  * `Cache-Control: private, max-age=60` など短め（必要ならis22側でメモリキャッシュ）

## A-3. 署名（DB不要で検証できる方式）

**HMAC-SHA256署名**にすると、URL検証でDB参照不要です。

* 署名対象（例）

  * `frame_uuid|drive_file_id|exp_unix|kind`
* `sig = base64url(hmac_sha256(secret, payload))`
* is22は受け取った exp/sig を検証し、期限内ならOK

> これなら「LLMにリンクを出させる」運用でも、リンクの寿命を短くできます。

## A-4. 追加の安全策（おすすめ）

* `sig` に **発行先クライアント識別**（例: セッションID）を混ぜる（可能なら）
* `/media/*` は **管理UIと同等の認証**を要求（またはVPN内限定）
* “案件（case）” は `retention_class=case` で **別TTL**/アクセス権（adminのみ）にする

---

# B. is22側：イベント化ロジック擬似コード（open→close / best_frame / cooldown）

## B-1. 設計方針

* `frames` は毎回保存（no_event含む）
* UIや検索の主役は `events`
* イベントは **「同一camera_idで、連続する検知を束ねる」**
* 連続抑制（通知/自動モーダル/LLM）をイベント単位で行う

## B-2. パラメータ例（初期値）

```text
POLL_INTERVAL_SEC = 60

EVENT_CLOSE_GRACE_SEC = 120        # 2分検知なしでclose
EVENT_MERGE_GAP_SEC   = 90         # 90秒以内なら同一イベントに寄せる（VPN遅延対策）
NOTIFY_COOLDOWN_SEC   = 300        # 同一カメラ同種は5分抑制
LLM_COOLDOWN_SEC      = 600        # 同一カメラは10分に1回まで
AUTO_MODAL_SEC        = 20         # 異常時モーダル表示秒
```

## B-3. キー概念

* `primary_event`（human/animal/hazard/camera_issue/unknown…）
* `tags[]`（enum）
* `severity`（0-3）
* `confidence`（0-1）
* `retention_class`（normal/quarantine/case）

## B-4. 擬似コード（フレーム処理の本体）

```pseudo
function process_frame(camera_id, captured_at, collector_status, diff_metrics, infer_result_opt):
    # 1) frames保存（必ず）
    frame = insert_frame(camera_id, captured_at, collector_status, diff_metrics, infer_result_opt)

    # collector_statusがNGなら offline判定などに寄せて終了
    if collector_status != "ok":
        maybe_mark_camera_offline(camera_id, captured_at)
        maybe_close_open_event_if_too_old(camera_id, captured_at)
        return frame

    # 2) 推論なし（差分が小さい等）
    if infer_result_opt is None:
        # no_eventとして記録済み（frame.detected=0, primary_event=none）
        maybe_close_open_event_if_grace_passed(camera_id, captured_at)
        return frame

    r = infer_result_opt

    # 3) 推論結果のタグを frame_tags に反映（検索のため）
    upsert_frame_tags(frame.frame_id, r.tags, schema_tag_group_map)

    # 4) retention判定（Drive隔離/保持）
    retention = classify_retention(camera_id, r.primary_event, r.tags, r.severity, r.unknown_flag)
    update_frame_retention(frame.frame_id, retention)

    # 5) イベント化（open/close）
    open_event = get_open_event(camera_id)  # state=open の events
    if r.detected == false or r.primary_event == "none":
        # 検知なし → openイベントがあれば、猶予を見てclose
        maybe_close_open_event_if_grace_passed(camera_id, captured_at)
        return frame

    # 検知あり
    if open_event exists:
        if can_merge(open_event, r, captured_at):
            # 同一イベントに寄せる
            update_event_with_frame(open_event, frame, r, retention)
        else:
            close_event(open_event, end_at = captured_at)
            new_event = open_new_event(camera_id, captured_at, r, retention, frame)
    else:
        new_event = open_new_event(camera_id, captured_at, r, retention, frame)

    # 6) best_frame選定
    # bestは「severity優先→confidence→画質（blur低い等）→新しい」
    maybe_update_best_frame(current_event, frame, r)

    # 7) Drive保存（フル画像）
    # ルール：detected or unknown なら保存、no_eventは保存しない
    if should_upload_full_to_drive(r, retention):
        drive_file_id = upload_full_image(camera_id, captured_at, retention)
        update_frame_drive_file(frame.frame_id, drive_file_id)
        insert_drive_objects(drive_file_id, frame.frame_id, retention)

    # 8) 通知（観測事実のみ）
    if should_notify(camera_id, r, retention):
        if cooldown_ok("notify", camera_id, r.primary_event, NOTIFY_COOLDOWN_SEC):
            text = build_notification_text(camera_id, captured_at, r)  # notify_allowed=trueタグのみで組み立て
            send_webhook(text, maybe_proxy_link(frame.frame_uuid))
            record_notification(frame.frame_id, current_event.event_id, payload)
            set_cooldown("notify", camera_id, r.primary_event)

    # 9) unknown時のみ VLMエスカレーション（任意）
    if r.unknown_flag == true or needs_human_confirmation(r):
        if cooldown_ok("llm", camera_id, "any", LLM_COOLDOWN_SEC):
            # 代表フレームだけ投げる（イベント単位で1回）
            if is_event_best_frame(frame, current_event):
                review = call_vision_llm(proxy_link(frame.frame_uuid))
                save_llm_review(frame.frame_id, current_event.event_id, review)
                set_cooldown("llm", camera_id, "any")

    return frame
```

## B-5. can_mergeの例（イベント統合条件）

```pseudo
function can_merge(open_event, r, captured_at):
    # 時間ギャップ
    if captured_at - open_event.last_seen_at > EVENT_MERGE_GAP_SEC:
        return false

    # primary_eventが違うなら基本別イベント
    if open_event.primary_event != r.primary_event:
        return false

    # 重大度が極端に違う場合も分離（例：hazardの種別が変わる等）
    # （必要なら tags による分岐）
    return true
```

## B-6. best_frame更新の例

```pseudo
function maybe_update_best_frame(event, frame, r):
    best = get_best_frame(event)

    score = (
        r.severity * 1000
        + (r.confidence or 0) * 100
        - (frame.blur_score or 0) * 0.1
    )

    best_score = compute_score(best)

    if score > best_score:
        set_event_best_frame(event, frame.frame_id)
```

---

# C. Drive隔離ルール（tag→quarantine/case）テーブル案

## C-1. 優先順位

1. **case**：手動（管理UIで「案件」紐付け） or 明示タグ（`retention.case_hold`）
2. **quarantine**：重要タグ/高severity/unknown
3. **normal**：それ以外

## C-2. ルールテーブル（初期案）

| 条件                                                    | retention_class | Drive保存  | フォルダ                     | 例保持    |
| ----------------------------------------------------- | --------------- | -------- | ------------------------ | ------ |
| 管理UIでcase_id付与 / `retention.case_hold`                | case            | 必ず       | `/case/{case_id}/...`    | 手動延長   |
| `hazard.*`（smoke_like/flame_like/water_*）             | quarantine      | 必ず       | `/camera-quarantine/...` | 60日    |
| `camera.offline` / `camera.moved` / `camera.occluded` | quarantine      | 必ず       | quarantine               | 60日    |
| `behavior.loitering`                                  | quarantine      | 必ず       | quarantine               | 60日    |
| `object_missing.suspected`                            | quarantine      | 必ず       | quarantine               | 60日    |
| `animal.deer` / `animal.boar`                         | quarantine      | 必ず       | quarantine               | 60日    |
| `unknown_flag=true`                                   | quarantine      | 推奨（代表のみ） | quarantine               | 30-60日 |
| `severity>=2`                                         | quarantine      | 必ず       | quarantine               | 60日    |
| `detected=true`（上記以外）                                 | normal          | 必ず       | `/camera-archive/...`    | 14日    |
| `detected=false`（no_event）                            | normal          | 保存しない    | -                        | -      |

## C-3. 実装のコツ

* `retention_class` は **frameとeventの両方**に持たせる

  * eventは「max（より強い方）」に寄せる（1枚でもhazardが出たらevent全体をquarantine）
* Drive隔離フォルダは「コピー」より「最初から隔離フォルダに入れる」ほうが楽

  * つまり `classify_retention()` の判定結果でアップロード先フォルダを変える

---

# D. APIクライアント雛形（Rust / Node）

以下は **is22 → is21** の呼び出しを想定したクライアントです。
（スキーマ更新 `PUT /v1/schema`、推論 `POST /v1/analyze`、ヘルスチェック）

---

## D-1. Rust（reqwest）雛形

> `Cargo.toml`（例）

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "multipart", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
time = { version = "0.3", features = ["serde", "formatting"] }
uuid = { version = "1", features = ["v4"] }
```

> クライアント例

```rust
use anyhow::{anyhow, Context, Result};
use reqwest::multipart;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeResponse {
    pub schema_version: String,
    pub camera_id: String,
    pub captured_at: String,
    pub analyzed: bool,
    pub detected: bool,
    pub primary_event: String,
    pub tags: Vec<String>,
    pub severity: u8,
    pub unknown_flag: bool,
    pub confidence: Option<f32>,
    pub processing_ms: Option<u32>,
}

#[derive(Clone)]
pub struct Is21Client {
    base_url: String,
    http: reqwest::Client,
}

impl Is21Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("reqwest client");
        Self { base_url: base_url.into(), http }
    }

    pub async fn healthz(&self) -> Result<()> {
        let url = format!("{}/healthz", self.base_url);
        let r = self.http.get(url).send().await?;
        if !r.status().is_success() {
            return Err(anyhow!("healthz failed: {}", r.status()));
        }
        Ok(())
    }

    pub async fn put_schema(&self, schema_envelope: serde_json::Value) -> Result<()> {
        let url = format!("{}/v1/schema", self.base_url);
        let r = self.http.put(url).json(&schema_envelope).send().await?;
        if !r.status().is_success() {
            let body = r.text().await.unwrap_or_default();
            return Err(anyhow!("put_schema failed: {} body={}", r.status(), body));
        }
        Ok(())
    }

    pub async fn analyze_jpeg_bytes(
        &self,
        camera_id: &str,
        captured_at_iso: &str,
        schema_version: &str,
        request_id: Option<&str>,
        jpeg: Vec<u8>,
    ) -> Result<AnalyzeResponse> {
        let url = format!("{}/v1/analyze", self.base_url);

        let image_part = multipart::Part::bytes(jpeg)
            .file_name("infer.jpg")
            .mime_str("image/jpeg")?;

        let mut form = multipart::Form::new()
            .text("camera_id", camera_id.to_string())
            .text("captured_at", captured_at_iso.to_string())
            .text("schema_version", schema_version.to_string())
            .part("infer_image", image_part);

        if let Some(rid) = request_id {
            form = form.text("request_id", rid.to_string());
        }

        let resp = self.http.post(url).multipart(form).send().await
            .context("post analyze")?;

        if resp.status().as_u16() == 429 {
            return Err(anyhow!("is21 overloaded (429)"));
        }
        if resp.status().as_u16() == 409 {
            return Err(anyhow!("schema mismatch (409)"));
        }
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("analyze failed: {} body={}", resp.status(), body));
        }

        let out: AnalyzeResponse = resp.json().await?;
        Ok(out)
    }
}
```

> 運用ポイント

* 429が返ったら is22側で **送信間引き（cameraごと最新1枚だけ）**にする
* 409（schema mismatch）が返ったら is22が `PUT /v1/schema` を再送する

---

## D-2. Node.js / Next.js（fetch + FormData）雛形

> Node18+/Nextのサーバ側で動く想定（ブラウザ側ではなく “is22バックエンド” で使用）

```ts
// is21Client.ts
import { readFile } from "node:fs/promises";

export type AnalyzeResponse = {
  schema_version: string;
  camera_id: string;
  captured_at: string;
  analyzed: boolean;
  detected: boolean;
  primary_event: string;
  tags: string[];
  severity: number;
  unknown_flag: boolean;
  confidence?: number;
  processing_ms?: number;
};

export class Is21Client {
  constructor(private baseUrl: string) {}

  async healthz(): Promise<void> {
    const r = await fetch(`${this.baseUrl}/healthz`);
    if (!r.ok) throw new Error(`healthz failed: ${r.status}`);
  }

  async putSchema(schemaEnvelope: any): Promise<void> {
    const r = await fetch(`${this.baseUrl}/v1/schema`, {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(schemaEnvelope),
    });
    if (!r.ok) {
      const body = await r.text().catch(() => "");
      throw new Error(`putSchema failed: ${r.status} ${body}`);
    }
  }

  async analyzeJpegBuffer(args: {
    cameraId: string;
    capturedAtIso: string;
    schemaVersion: string;
    requestId?: string;
    jpegBuffer: Buffer;
  }): Promise<AnalyzeResponse> {
    const fd = new FormData();
    if (args.requestId) fd.set("request_id", args.requestId);
    fd.set("camera_id", args.cameraId);
    fd.set("captured_at", args.capturedAtIso);
    fd.set("schema_version", args.schemaVersion);

    // NodeのFormDataにBlobで渡す
    const blob = new Blob([args.jpegBuffer], { type: "image/jpeg" });
    fd.set("infer_image", blob, "infer.jpg");

    const r = await fetch(`${this.baseUrl}/v1/analyze`, {
      method: "POST",
      body: fd,
    });

    if (r.status === 429) throw new Error("is21 overloaded (429)");
    if (r.status === 409) throw new Error("schema mismatch (409)");
    if (!r.ok) {
      const body = await r.text().catch(() => "");
      throw new Error(`analyze failed: ${r.status} ${body}`);
    }
    return (await r.json()) as AnalyzeResponse;
  }
}
```

---

# E. 追加：通知/LLM/自動モーダルのクールダウン実装（最小案）

DBに恒久保存してもいいですが、まずは is22のプロセス内で十分です。

* キー例：

  * notify: `notify:{camera_id}:{primary_event}`
  * llm: `llm:{camera_id}`
  * modal: `modal:{camera_id}`
* 値：`last_time_unix`

永続化が必要なら `settings` テーブルに JSON で持つか、専用テーブルを追加します。

---
