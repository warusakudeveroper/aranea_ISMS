# PTZctrl 完全実装計画書

**作成日**: 2026-01-19
**対象**: PTZctrl_BasicInstructions.md 未実装項目の完全実装
**ステータス**: 実装完了 (2026-01-19)

---

## 1. 設計是正事項

### 1.1 PTZトグルボタンの廃止

**現状の問題**:
- LiveViewModalでPTZモードボタンをクリックしないとPTZ操作できない
- しかし、CameraDetailModalに「ptz_disabled」チェックボックスが存在
- 誤操作防止の手段が二重に存在し、冗長

**是正方針**:
- PTZトグルボタンを廃止
- `ptz_supported=true && ptz_disabled=false` のカメラでモーダルを開いたら即座にPTZオーバーレイを表示
- 「誤操作防止したい」ユーザーはCameraDetailModalでptz_disabledをONに設定

**変更ファイル**: `LiveViewModal.tsx`
```diff
- const [showPtzControls, setShowPtzControls] = useState(false)
+ // PTZ表示は ptz_supported && !ptz_disabled で自動決定
+ const showPtzControls = camera?.ptz_supported && !camera?.ptz_disabled
```

---

## 2. 未実装項目一覧（MECE）

| # | 項目 | 重要度 | Phase |
|---|------|--------|-------|
| 1 | PTZバックエンドONVIF実体実装 | 最重要 | 1 |
| 2 | PTZルートLease認証有効化 | 最重要 | 1 |
| 3 | PTZトグルボタン廃止 | 高 | 2 |
| 4 | LiveViewModalサイズ130%達成 | 中 | 2 |
| 5 | PTZ操作フィードバック表示 | 中 | 2 |
| 6 | PTZ非対応カメラのグレーアウト | 低 | 2 |
| 7 | SuggestPane 3台制限修正 | 高 | 3 |
| 8 | SuggestPane 重複防止修正 | 高 | 3 |
| 9 | AccessAbsorber連携 | 中 | 4 |

---

## 3. Phase 1: Backend ONVIF実体実装

### 3.1 目的
PTZ操作API呼び出し時に実際にカメラが動作するようにする

### 3.2 対象ファイル
- `src/ptz_controller/service.rs`
- `src/web_api/ptz_routes.rs`
- `Cargo.toml`（依存追加）

### 3.3 実装内容

#### 3.3.1 Tapo専用ONVIF制御（優先）

Tapoカメラは非標準ONVIFのため、専用実装が必要:

```rust
// src/ptz_controller/tapo_ptz.rs (新規)

use reqwest::Client;
use base64::Engine;

/// Tapo ONVIF PTZ制御
pub struct TapoPtzClient {
    endpoint: String,
    username: String,
    password: String,
    client: Client,
}

impl TapoPtzClient {
    pub async fn continuous_move(&self, pan: f32, tilt: f32, _zoom: f32) -> Result<(), Error> {
        let soap_body = format!(r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
              <s:Body>
                <ContinuousMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
                  <ProfileToken>profile_1</ProfileToken>
                  <Velocity>
                    <PanTilt x="{}" y="{}" xmlns="http://www.onvif.org/ver10/schema"/>
                  </Velocity>
                </ContinuousMove>
              </s:Body>
            </s:Envelope>
        "#, pan, tilt);

        self.send_soap_request(&soap_body).await
    }

    pub async fn stop(&self) -> Result<(), Error> {
        let soap_body = r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
              <s:Body>
                <Stop xmlns="http://www.onvif.org/ver20/ptz/wsdl">
                  <ProfileToken>profile_1</ProfileToken>
                  <PanTilt>true</PanTilt>
                  <Zoom>true</Zoom>
                </Stop>
              </s:Body>
            </s:Envelope>
        "#;

        self.send_soap_request(soap_body).await
    }

    pub async fn goto_home(&self) -> Result<(), Error> {
        let soap_body = r#"
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
              <s:Body>
                <GotoHomePosition xmlns="http://www.onvif.org/ver20/ptz/wsdl">
                  <ProfileToken>profile_1</ProfileToken>
                </GotoHomePosition>
              </s:Body>
            </s:Envelope>
        "#;

        self.send_soap_request(soap_body).await
    }

    async fn send_soap_request(&self, body: &str) -> Result<(), Error> {
        // WS-Security UsernameToken 認証付きリクエスト
        let nonce = generate_nonce();
        let created = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let digest = calculate_password_digest(&nonce, &created, &self.password);

        let security_header = format!(r#"
            <s:Header>
              <Security xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
                <UsernameToken>
                  <Username>{}</Username>
                  <Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">{}</Password>
                  <Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">{}</Nonce>
                  <Created xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">{}</Created>
                </UsernameToken>
              </Security>
            </s:Header>
        "#, self.username, digest, base64::engine::general_purpose::STANDARD.encode(&nonce), created);

        // bodyにheaderを挿入
        let full_body = body.replace("<s:Body>", &format!("{}<s:Body>", security_header));

        let ptz_url = format!("{}/onvif/ptz_service", self.endpoint.trim_end_matches("/onvif/device_service"));

        self.client
            .post(&ptz_url)
            .header("Content-Type", "application/soap+xml")
            .body(full_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
```

#### 3.3.2 service.rs 修正

```rust
// execute_ptz_move を実装に置換
pub async fn execute_ptz_move(&self, camera: &Camera, request: &PtzMoveRequest) -> Result<(), Error> {
    // 1. rotation適用
    let actual_direction = request.direction.apply_rotation(camera.rotation);

    // 2. Tapoカメラ判定
    if camera.family == "tapo" {
        let endpoint = camera.onvif_endpoint.as_ref()
            .ok_or_else(|| Error::Config("ONVIF endpoint not configured".into()))?;
        let username = camera.rtsp_username.as_ref()
            .ok_or_else(|| Error::Config("RTSP username not configured".into()))?;
        let password = camera.rtsp_password.as_ref()
            .ok_or_else(|| Error::Config("RTSP password not configured".into()))?;

        let client = TapoPtzClient::new(endpoint, username, password);

        // 3. 方向→速度ベクトル変換
        let (pan, tilt) = match actual_direction {
            PtzDirection::Up => (0.0, request.speed),
            PtzDirection::Down => (0.0, -request.speed),
            PtzDirection::Left => (-request.speed, 0.0),
            PtzDirection::Right => (request.speed, 0.0),
        };

        // 4. 実行
        client.continuous_move(pan, tilt, 0.0).await?;

        // 5. Nudgeモードなら自動停止
        if request.mode == PtzMode::Nudge {
            tokio::time::sleep(tokio::time::Duration::from_millis(request.duration_ms as u64)).await;
            client.stop().await?;
        }

        return Ok(());
    }

    // 他メーカーは未実装
    Err(Error::NotSupported(format!("PTZ not implemented for family: {}", camera.family)))
}
```

#### 3.3.3 Lease認証有効化

```rust
// ptz_routes.rs:20-24 コメント解除
let lease = state.admission_controller.get_lease_by_id(&request.lease_id).await;
if lease.is_none() {
    return Err((StatusCode::UNAUTHORIZED, Json(PtzResponse::error("Invalid lease"))));
}
```

### 3.4 テスト項目
- [ ] Tapo C210/C220でPTZ操作が実際に動作すること
- [ ] 無効なLease IDでAPIを叩くと401が返ること
- [ ] rotation=180のカメラで「上」を押すと実際に「下」に動くこと
- [ ] Nudgeモードで指定時間後に自動停止すること

---

## 4. Phase 2: Frontend UI修正

### 4.1 PTZトグルボタン廃止 & 自動表示

**変更ファイル**: `LiveViewModal.tsx`

```diff
- const [showPtzControls, setShowPtzControls] = useState(false)
+ // PTZ対応かつ無効化されていなければ自動表示
+ const showPtzControls = useMemo(() => {
+   return camera?.ptz_supported === true && camera?.ptz_disabled !== true && leaseId !== null
+ }, [camera?.ptz_supported, camera?.ptz_disabled, leaseId])
```

PTZトグルボタン（Move3dアイコン）のコード削除:
```diff
- {camera.ptz_supported && !camera.ptz_disabled && (
-   <Button
-     variant="ghost"
-     size="icon"
-     onClick={() => setShowPtzControls(!showPtzControls)}
-     ...
-   >
-     <Move3d ... />
-   </Button>
- )}
```

### 4.2 LiveViewModalサイズ130%達成

**変更ファイル**: `LiveViewModal.tsx:150`

```diff
- <DialogContent className="max-w-5xl p-0 overflow-hidden">
+ <DialogContent className="max-w-6xl p-0 overflow-hidden">
```

計算:
- max-w-4xl = 896px（元）
- max-w-6xl = 1152px
- 1152 / 896 = 128.5% ≒ 130%

### 4.3 PTZ操作フィードバック表示

**変更ファイル**: `ptz/PtzOverlay.tsx`

```tsx
// 追加: 操作フィードバックコンポーネント
const [feedback, setFeedback] = useState<{direction: string; action: string} | null>(null)

const showFeedback = (direction: PtzDirection, action: string) => {
  const labels: Record<PtzDirection, string> = {
    up: "↑ pan up",
    down: "↓ pan down",
    left: "← pan left",
    right: "→ pan right",
  }
  setFeedback({ direction: labels[direction], action })
  setTimeout(() => setFeedback(null), 1500)
}

// JSX追加（オーバーレイ中央付近）
{feedback && (
  <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2
                  bg-black/70 text-white px-3 py-1 rounded text-sm font-mono
                  animate-fade-in">
    {feedback.direction}
  </div>
)}
```

### 4.4 PTZ非対応カメラのグレーアウト表示

**変更ファイル**: `CameraDetailModal.tsx:747`

```diff
- {camera.ptz_supported && (
-   <Section title="PTZ能力" icon={<Move3d className="h-4 w-4" />}>
+ <Section
+   title="PTZ能力"
+   icon={<Move3d className={cn("h-4 w-4", !camera.ptz_supported && "text-muted-foreground")} />}
+   disabled={!camera.ptz_supported}
+ >
+   {!camera.ptz_supported ? (
+     <p className="text-sm text-muted-foreground">このカメラはPTZに対応していません</p>
+   ) : (
      // 既存のPTZ設定UI
+   )}
+ </Section>
```

### 4.5 テスト項目
- [ ] PTZ対応カメラでモーダルを開くと即座にPTZオーバーレイが表示されること
- [ ] ptz_disabled=trueのカメラではPTZオーバーレイが表示されないこと
- [ ] PTZ非対応カメラでは「PTZ能力」セクションがグレーアウト表示されること
- [ ] 方向ボタン押下時に「← pan left」等のフィードバックが表示されること
- [ ] モーダルサイズが従来比130%程度に拡大されていること

---

## 5. Phase 3: SuggestPane修正

### 5.1 3台制限の厳格化

**問題**: exiting状態のカメラも表示されるため4台以上表示される

**変更ファイル**: `SuggestPane.tsx:267-269`

```diff
  const visibleCameras = useMemo(() => {
-   return onAirCameras
+   // exiting状態を除外し、最大3台に制限
+   return onAirCameras
+     .filter(c => !c.isExiting)
+     .slice(0, 3)
  }, [onAirCameras])
```

### 5.2 同一カメラ重複防止の修正

**問題**: `!c.isExiting` 条件により、exiting状態のカメラは重複チェックから除外される

**変更ファイル**: `SuggestPane.tsx:96-98`

```diff
  const existingIndex = prev.findIndex(c =>
-   (c.lacisId === eventLacisId || c.cameraId === camera.camera_id) && !c.isExiting
+   (c.lacisId === eventLacisId || c.cameraId === camera.camera_id)
  )
+
+ // 既存エントリがある場合（exiting含む）
+ if (existingIndex !== -1) {
+   const existing = prev[existingIndex]
+   if (existing.isExiting) {
+     // exiting中なら即座にexitを完了させ、新規追加
+     const newList = prev.filter((_, i) => i !== existingIndex)
+     // 新規追加処理へ
+   } else {
+     // 通常の更新処理
+   }
+ }
```

### 5.3 テスト項目
- [ ] サジェストビューに4台以上同時表示されないこと
- [ ] 同一カメラが複数表示されないこと
- [ ] exiting状態のカメラと同じカメラの新イベントが来た場合、正しくハンドリングされること

---

## 6. Phase 4: AccessAbsorber連携

### 6.1 目的
クリックモーダルとサジェストのストリーム競合を管理し、ユーザーフィードバックを提供

### 6.2 実装内容

#### 6.2.1 Backend: AccessAbsorber統合API

```rust
// src/web_api/stream_routes.rs (新規または追加)

/// ストリーム要求（AccessAbsorber経由）
#[derive(Deserialize)]
pub struct StreamRequest {
    camera_id: String,
    consumer: String,  // "LiveViewModal" | "SuggestPane"
    quality: String,   // "main" | "sub"
}

#[derive(Serialize)]
pub struct StreamResponse {
    ok: bool,
    stream_url: Option<String>,
    denied_reason: Option<String>,      // 拒否理由
    user_message: Option<String>,       // ユーザー向けメッセージ
}

pub async fn request_stream(
    State(state): State<AppState>,
    Json(req): Json<StreamRequest>,
) -> Json<StreamResponse> {
    // AccessAbsorberで競合チェック
    let result = state.access_absorber.request_connection(
        &req.camera_id,
        &req.consumer,
        &req.quality,
    ).await;

    match result {
        Ok(stream_url) => Json(StreamResponse {
            ok: true,
            stream_url: Some(stream_url),
            denied_reason: None,
            user_message: None,
        }),
        Err(e) => Json(StreamResponse {
            ok: false,
            stream_url: None,
            denied_reason: Some(e.code()),
            user_message: Some(e.to_user_message()),
        }),
    }
}
```

#### 6.2.2 Frontend: SuggestPane競合フィードバック

```tsx
// SuggestPane.tsx 追加

const [deniedMessage, setDeniedMessage] = useState<string | null>(null)

const requestStream = async (cameraId: string) => {
  const response = await fetch(`${API_BASE_URL}/api/stream/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      camera_id: cameraId,
      consumer: "SuggestPane",
      quality: "sub",
    }),
  })
  const result = await response.json()

  if (!result.ok) {
    // 競合フィードバック表示
    setDeniedMessage(result.user_message || "ストリーム取得に失敗しました")
    setTimeout(() => setDeniedMessage(null), 3000)
    return null
  }

  return result.stream_url
}

// JSX追加
{deniedMessage && (
  <div className="absolute bottom-4 left-1/2 -translate-x-1/2
                  bg-amber-500 text-white px-4 py-2 rounded shadow-lg
                  animate-slide-up">
    {deniedMessage}
  </div>
)}
```

### 6.3 テスト項目
- [ ] LiveViewModalでストリーム中のカメラがSuggestPaneに出た場合、適切なフィードバックが表示されること
- [ ] フィードバックは3秒後に自動で消えること
- [ ] AccessAbsorberの接続制限に達した場合、適切なエラーメッセージが表示されること

---

## 7. 実装順序とスケジュール

```
Phase 1 (Backend ONVIF) ──┬──> Phase 2 (Frontend UI) ──> Phase 3 (SuggestPane)
                          │
                          └──> Phase 4 (AccessAbsorber)
```

Phase 1 が完了しないとPTZ機能全体が動作しないため、最優先で実施。
Phase 2-4 は並行実施可能。

---

## 8. 受け入れ条件チェックリスト

### 8.1 機能要件
- [ ] Tapo PTZカメラで実際にカメラが動作すること
- [ ] PTZトグルボタンなしで即座にPTZ操作可能なこと
- [ ] ptz_disabledチェックでPTZ UIを非表示にできること
- [ ] 方向ボタン押下時にフィードバック表示されること
- [ ] サジェストビューが最大3台であること
- [ ] 同一カメラが重複表示されないこと
- [ ] ストリーム競合時にユーザーフィードバックが表示されること

### 8.2 非機能要件
- [ ] PTZ操作のレスポンス時間が500ms以内であること
- [ ] モーダルサイズが従来比130%程度であること

### 8.3 テスト実施
- [ ] 実機テスト（Tapo C210/C220）完了
- [ ] フロントエンドE2Eテスト完了
- [ ] 競合シナリオテスト完了

---

## 9. リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| TapoのONVIF仕様変更 | PTZ動作不能 | ファームウェアバージョン固定推奨 |
| WS-Security実装の複雑さ | 開発遅延 | 既存ライブラリ活用検討 |
| AccessAbsorber未テスト | 競合処理失敗 | スタブモードでの段階的実装 |

---

## 10. 承認

- [ ] 設計レビュー完了
- [ ] 実装開始承認

**承認者**: _______________
**日付**: _______________
