# ffmpegプロセスリーク障害 調査レビュー

**作成日**: 2026-01-05 08:00 JST
**更新日**: 2026-01-05 09:30 JST
**関連Issue**: [#73](https://github.com/warusakudeveroper/aranea_ISMS/issues/73)
**詳細レポート**: [INCIDENT_REPORT_20260105_ffmpeg_leak.md](../../../docs/is22/INCIDENT_REPORT_20260105_ffmpeg_leak.md)

---

## 1. タイムゾーン問題の発見

### 1.1 環境確認結果

| 環境 | タイムゾーン | 現在時刻 |
|------|-------------|----------|
| is22サーバー | **UTC** | 2026-01-04 22:49:46 |
| クライアント | **JST** | 2026-01-05 07:49:45 |

**差分**: 9時間

### 1.2 影響

- ログ解析時に時刻の誤認識が発生
- 初回レポートで「22:00頃」と記載したが、実際はJST 07:50頃の観測
- サーバーログのタイムスタンプはすべてUTC

### 1.3 推奨対応

1. **is22サーバーのタイムゾーン変更検討**: UTCのままとするか、JSTに統一するか
2. **ログ出力時のタイムゾーン明示**: `2026-01-05T07:50:00+09:00` 形式

---

## 2. プロセスリーク進行状況

### 2.1 時系列データ

| 解析時刻 (JST) | UTC時刻 | ゾンビffmpeg数 | 125.19 | 125.12 | 増加 |
|----------------|---------|----------------|--------|--------|------|
| 07:24 | 22:24 | 25 | 22 | 1 | - |
| 07:51 | 22:51 | 37 | 32 | 1 | +12 |
| 08:28 | 23:28 | 39 | 33 | 4 | +2 |

**増加ペースの変化**:
- 07:24→07:51: 27分で+12プロセス（約2.2分/プロセス）- 急速蓄積
- 07:51→08:28: 37分で+2プロセス（約18.5分/プロセス）- 減速

**時間帯別蓄積数（UTC）**:
- 18時台: 1プロセス（最古、125.12）
- 21時台: 17プロセス（125.19への集中開始）
- 22時台: 16プロセス（125.19継続）
- 23時台: 5プロセス（125.12への新規蓄積発生）

### 2.2 最古プロセスの生存時間

```
PID 1504968
起動: Jan 4 18:10:04 UTC = Jan 5 03:10:04 JST
接続先: 192.168.125.12:554/stream2
```

| 解析時点 | 生存時間 |
|----------|----------|
| 07:51 JST | 4時間41分 |
| 08:28 JST | 5時間18分（ELAPSED実測値: 05:19:00）|

**重要**: 初期レポートの「6時間40分」は計算誤りでした。正しくは上記の通り。

---

## 3. 連動性確認

### 3.1 is22起動〜障害発生の相関

```
01:23 JST - is22起動
        ↓ (約2時間)
03:10 JST - 最初のゾンビ発生（192.168.125.12）
        ↓ (約3時間、この間もゾンビ発生の可能性）
06:07 JST - 192.168.125.19への大量蓄積開始
        ↓ (継続)
07:51 JST - 37プロセス蓄積確認
```

### 3.2 推定される障害トリガー

1. **カメラ応答遅延/無応答**: 特に192.168.125.19 (Tapo C110)
2. **RTSP接続タイムアウト**: 30秒で発動するが、ffmpegプロセスは生存
3. **ポーリング間隔**: 約2分間隔でリトライ → ゾンビ蓄積

---

## 4. コード詳細分析

### 4.1 polling_orchestratorのタイムアウト処理フロー

**ファイル**: `src/polling_orchestrator/mod.rs`

```rust
// Line 47: タイムアウト値の定義
const SNAPSHOT_TIMEOUT_MS: u64 = 30000;  // 30秒

// Lines 738-768: タイムアウト処理
let capture_result = match tokio::time::timeout(
    Duration::from_millis(SNAPSHOT_TIMEOUT_MS),
    snapshot_service.capture(camera),
)
.await
{
    Ok(Ok(result)) => result,
    Ok(Err(e)) => { /* エラー処理 */ }
    Err(_) => {
        // タイムアウト発生
        // ただし、ffmpegプロセスは生存したまま！
        tracing::warn!(
            camera_id = %camera.camera_id,
            camera_ip = %camera.ip_address,
            preset_id = %preset.id,
            timeout_ms = SNAPSHOT_TIMEOUT_MS,
            "Snapshot capture timeout"
        );
        return Err(crate::error::Error::Internal(format!(
            "Snapshot capture timeout ({}ms) for camera {}",
            SNAPSHOT_TIMEOUT_MS, camera.camera_id
        )));
    }
}
```

**問題点**:
- `tokio::time::timeout`はFutureをキャンセルするだけ
- 内部で起動されたffmpegプロセスは終了されない
- `Command::output()` が使用されており、子プロセスのPIDを追跡していない

### 4.2 snapshot_serviceの問題箇所

**ファイル**: `src/snapshot_service/mod.rs` (Lines 270-288)

```rust
async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args([
            "-rtsp_transport", "tcp",
            "-i", rtsp_url,
            "-frames:v", "1",
            "-f", "image2pipe",
            "-vcodec", "mjpeg",
            "-loglevel", "error",
            "-y",
            "-",
        ])
        .output()  // ← ここが問題: タイムアウト管理なし
        .await
        .map_err(|e| Error::Internal(format!("ffmpeg execution failed: {}", e)))?;

    // ...
}
```

**問題点**:
- `Command::output()`は完了を待つのみ
- 親プロセス（is22）がタイムアウトしても子プロセス（ffmpeg）は生存
- `spawn()` + 明示的 `kill()` が必要

---

## 5. go2rtc統合状態分析

### 5.1 なぜ視聴者がいないとffmpegを使うのか

**ファイル**: `src/snapshot_service/mod.rs` (Lines 202-244)

```rust
async fn try_go2rtc_snapshot(&self, stream_id: &str) -> Result<Option<Vec<u8>>> {
    // go2rtcのストリーム情報を取得
    let streams: serde_json::Value = resp.json().await?;

    // アクティブなproducerがあるかチェック
    let has_active_producer = stream
        .get("producers")
        .and_then(|p| p.as_array())
        .map(|producers| {
            producers.iter().any(|p| {
                // recv > 0 = データを受信中 = 視聴者がいる
                p.get("recv")
                    .and_then(|r| r.as_u64())
                    .unwrap_or(0) > 0
            })
        })
        .unwrap_or(false);

    if !has_active_producer {
        // 視聴者がいない場合はNoneを返す → ffmpegにフォールバック
        return Ok(None);
    }

    // 視聴者がいる場合のみgo2rtcのframe.jpeg APIを使用
    let frame_url = format!("{}/api/frame.jpeg?src={}", GO2RTC_BASE_URL, stream_id);
    // ...
}
```

**設計の意図**:
- go2rtcはライブビュー再生専用として実装
- 視聴者がいる時のみRTSP接続をgo2rtcに委譲

**実際の問題**:
- 通常のポーリング（視聴者なし）は常にffmpegを使用
- ffmpegプロセスリークの問題はgo2rtc統合後も残存

### 5.2 StreamGateway.remove_source()の未使用

**ファイル**: `src/stream_gateway/mod.rs` (Lines 111-125)

```rust
/// Remove a stream source
pub async fn remove_source(&self, name: &str) -> Result<()> {
    let url = format!("{}/api/streams?name={}", self.base_url, name);
    let resp = self.client.delete(&url).send().await?;
    // ...
}
```

**問題**: この関数は存在するが、**どこからも呼び出されていない**。
サブネット巡回終了時のクリーンアップが未実装。

---

## 6. 設計ドキュメントとの比較

### 6.1 設計意図（is21_go2rtc_integration_tasks.md より）

```
期待される動作:
1. サブネット巡回終了時にWSを切断
2. コンストラクト-ディストラクト型でクリーンなプロセス巡回
3. 再生中はセッション継続
4. 再生終了後は通常ループに戻る
5. クールダウン後に接続再確立して巡回
```

### 6.2 実装との乖離

| 設計意図 | 実装状態 | 備考 |
|----------|----------|------|
| サブネット巡回終了時にWS切断 | **未実装** | remove_source()未使用 |
| コンストラクト-ディストラクト型 | **未実装** | プロセス管理なし |
| go2rtc優先利用 | **部分実装** | 視聴者がいる時のみ |
| ffmpegプロセス管理 | **未実装** | kill処理なし |

### 6.3 RtspManagerの限界

**ファイル**: `src/rtsp_manager/mod.rs`

```rust
/// RTSPアクセスリース - Dropで自動解放
pub struct RtspLease {
    camera_id: String,
    _guard: tokio::sync::OwnedMutexGuard<()>,  // 論理的ロックのみ
}

impl Drop for RtspLease {
    fn drop(&mut self) {
        // ロック解放のみ、ffmpegプロセスは殺さない
        tracing::debug!(camera_id = %self.camera_id, "RTSP access released");
    }
}
```

**問題**: RtspManagerは論理的なアクセス制御のみ。実際のプロセスライフサイクル管理は行っていない。

---

## 7. カメラ別障害パターン分析

### 7.1 なぜ192.168.125.19に集中したか

**観測データ（直近3時間のログ分析）**:

| 時刻 (UTC) | イベント | snapshot_ms |
|------------|----------|-------------|
| 20:07:17 | **TIMEOUT** | 30000+ |
| 20:08:59 | SUCCESS | 25675 |
| 20:10:48 | SUCCESS | 16881 |
| 20:12:36 | SUCCESS | 16755 |
| 20:14:19 | SUCCESS | 27634 |
| 20:16:21 | **TIMEOUT** | 30000+ |
| 20:18:20 | SUCCESS | 28131 |
| ... | ... | ... |

**パターン分析**:

1. **カメラ特性**: Tapo C110 は応答が遅い（snapshot_ms: 12〜28秒）
2. **タイムアウト頻度**: 約2分おきにタイムアウト発生（30秒閾値ギリギリ）
3. **蓄積メカニズム**:
   - 成功時: ffmpegは正常終了（リーク発生しない）
   - タイムアウト時: ffmpegが残存（リーク発生）
4. **なぜこのカメラに集中**:
   - 他のカメラ: 2-5秒で応答 → タイムアウトほぼなし
   - 125.19: 15-28秒で応答 → タイムアウト頻発

### 7.2 他カメラの状況

```
ffmpegプロセス分布（07:51 JST時点）:
  32 192.168.125.19  ← Tapo C110、最も遅い
   1 192.168.125.79  ← Tapo、偶発的タイムアウト
   1 192.168.125.62
   1 192.168.125.45
   1 192.168.125.17
   1 192.168.125.12  ← 最古（6時間40分生存）
```

---

## 8. 自動再起動ログ確認

### 8.1 is22起動履歴

```
UTC時刻           | JST時刻         | イベント
------------------|-----------------|------------------
Jan 4 12:31:33    | Jan 4 21:31:33  | Started is22
Jan 4 15:34:02    | Jan 5 00:34:02  | Started is22
Jan 4 15:43:45    | Jan 5 00:43:45  | Started is22
Jan 4 16:04:04    | Jan 5 01:04:04  | Started is22
Jan 4 16:23:14    | Jan 5 01:23:14  | Started is22 ← 現行
```

### 8.2 03:00-03:30 JST（18:00-18:30 UTC）のログ

**結論**: この時間帯にis22の再起動はなし。通常のポーリング動作のみ。

最初のゾンビプロセスが発生した03:10 JSTは、
is22起動（01:23 JST）から約2時間後。
この間に蓄積が開始されたと推定。

---

## 9. 設計レビュー指摘事項（更新）

### 9.1 欠落機能

| 機能 | 設計意図 | 現状 | 影響度 |
|------|----------|------|--------|
| ffmpegプロセスkill | タイムアウト時に終了 | **未実装** | 致命的 |
| サブネット巡回終了時クリーンアップ | RTSP接続を切断 | **未実装** | 高 |
| go2rtc常時利用 | RTSP競合回避 | **視聴者がいる時のみ** | 中 |
| コンストラクト-ディストラクト | クリーンな巡回 | **未実装** | 中 |
| プロセス監視 | ゾンビ検出・kill | **未実装** | 中 |

### 9.2 根本原因サマリー

1. **直接原因**: `Command::output()`がタイムアウト後もffmpegを生存させる
2. **設計原因**: プロセスライフサイクル管理の欠如
3. **悪化要因**: 特定カメラ（125.19）の応答遅延によるタイムアウト頻発
4. **波及影響**: Tapoカメラ接続制限（2-4接続）を超過し全カメラ接続失敗

---

## 10. 次のステップ

### 10.1 即時対応（コード修正必要）

- [ ] `capture_rtsp()`で`Command::spawn()` + `child.kill()`を使用
- [ ] タイムアウト時に子プロセスを明示的にkill
- [ ] ゾンビプロセス定期クリーンアップ実装（5分超生存プロセスをkill）

### 10.2 中期対応

- [ ] サブネット巡回終了時に`StreamGateway.remove_source()`を呼び出し
- [ ] go2rtc常時利用化（ポーリング時もgo2rtc frame.jpeg APIを使用）
- [ ] RtspManagerにプロセス管理機能を追加

### 10.3 運用対応

- [ ] is22サーバーのタイムゾーン設定検討（JSTへの変更）
- [ ] ffmpegプロセス数監視アラート設定（閾値: 5プロセス超）
- [ ] 応答遅延カメラ（125.19）のタイムアウト値調整検討（30秒→45秒）

---

## 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 08:00 | 初版作成 |
| 2026-01-05 09:30 | コード詳細分析、go2rtc統合分析、カメラ別パターン、再起動ログ確認追加 |
| 2026-01-05 08:30 | セクション2.1/2.2詳細検証・修正（生存時間計算誤り訂正、08:28時点データ追加）|

---

**レビュー担当**: Claude Code
**ステータス**: 詳細調査完了、修正実装待ち
