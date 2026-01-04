# 障害解析レポート: ffmpegプロセスリークによるカメラ接続全喪失

**発生日時**: 2026-01-05 22:00頃
**報告日時**: 2026-01-05 22:30
**影響範囲**: is22全カメラ（特に192.168.125サブネット）
**重大度**: Critical

---

## 1. 障害概要

is22のポーリング処理において、ffmpegプロセスがタイムアウト後も終了せずに蓄積し続け、Tapoカメラの接続制限（2-4接続）を超過。結果として大半のカメラへの接続が失敗する状態に陥った。

---

## 2. タイムライン

| 時刻 | イベント |
|------|----------|
| 16:23:14 | is22起動（PID 1468583） |
| 18:10:04 | **最初のゾンビffmpeg発生**（192.168.125.12、PID 1504968） |
| 21:07:42 | 192.168.125.19へのffmpegプロセス蓄積開始 |
| 21:07〜22:24 | 192.168.125.19へのffmpegが22個蓄積 |
| 22:24現在 | 合計25個のゾンビffmpegプロセス |

---

## 3. 診断データ

### 3.1 ffmpegプロセス状況

```
=== ffmpegの接続先カメラ集計 ===
     22 192.168.125.19    ← Tapo C110 - 接続枯渇
      1 192.168.125.79    ← Tapo
      1 192.168.125.62    ← 接続中ハング
      1 192.168.125.12    ← 最古（4時間14分）

=== ffmpegプロセス数 ===
25
```

### 3.2 最古のプロセス

```
PID 1504968, started Sun Jan  4 18:10:04 2026
Parent: 1468583 (is22 camserver)
Target: 192.168.125.12:554
Running time: 4時間14分
```

### 3.3 ネットワーク接続状態

```
=== RTSP接続状態（ポート554）===
     25 192.168.125.246 (is22サーバー)
```

---

## 4. 根本原因分析

### 4.1 コード上の問題

**ファイル**: `src/snapshot_service/mod.rs` (270-288行)

```rust
async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args([...])
        .output()  // ← タイムアウト処理なし、子プロセス管理なし
        .await
        .map_err(...)?;
    ...
}
```

**ファイル**: `src/polling_orchestrator/mod.rs` (738-768行)

```rust
let capture_result = match tokio::time::timeout(
    Duration::from_millis(SNAPSHOT_TIMEOUT_MS),  // 30秒
    snapshot_service.capture(camera),
)
.await
{
    Ok(Ok(result)) => result,
    Err(_) => {
        // タイムアウト - しかしffmpegプロセスは生存
        return Err(...);
    }
}
```

### 4.2 問題の連鎖

1. ffmpegがRTSP接続でハング（カメラ無応答など）
2. 30秒タイムアウト発生 → Futureがキャンセル
3. **ffmpegプロセスは生存したまま**（RTSPコネクション保持）
4. 次のポーリングサイクルで新しいffmpegが起動
5. Tapoカメラは最大2-4接続制限 → 接続スロット枯渇
6. 新規リクエストは「Operation not permitted」で失敗
7. 全カメラへの接続が波及的に失敗

### 4.3 設計上の問題

**欠落している機能**:

1. **サブネット巡回終了時のクリーンアップ**: 実装されていない
2. **go2rtcストリーム解除**: `StreamGateway.remove_source()`は存在するが未使用
3. **ffmpegプロセスのライフサイクル管理**: RtspManagerは論理的ロックのみ
4. **コンストラクト-ディストラクト型巡回**: 設計意図どおり実装されていない

**期待される動作**（設計意図）:
- 巡回終了時にWebSocket/RTSP接続を切断
- クールダウン期間中はクリーンな状態
- 再生中はセッション継続
- 再生終了後に通常ループに戻る
- 次の巡回開始時に接続を再確立

---

## 5. RtspManager分析

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

**問題**: RtspLeaseがDropされてもffmpegプロセスは終了しない。

---

## 6. 修正方針

### 6.1 即時対応（短期）

1. `capture_rtsp`で`Command::spawn() + kill()`を使用
2. タイムアウト時に子プロセスを明示的にkill

```rust
async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
    let mut child = Command::new("ffmpeg")
        .args([...])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    match tokio::time::timeout(
        Duration::from_secs(self.ffmpeg_timeout),
        child.wait_with_output()
    ).await {
        Ok(output) => { ... }
        Err(_) => {
            // タイムアウト - 子プロセスを明示的にkill
            let _ = child.kill().await;
            return Err(...);
        }
    }
}
```

### 6.2 中期対応

1. サブネット巡回終了時にgo2rtcストリーム解除を実装
2. RtspManagerにプロセス管理機能を追加
3. 定期的なゾンビプロセスチェック＆クリーンアップ

### 6.3 長期対応

1. go2rtc経由のスナップショット取得を優先（ffmpeg使用を減らす）
2. コンストラクト-ディストラクト型巡回の完全実装
3. プロセスリソース監視とアラート

---

## 7. 影響を受けたカメラ

| カメラIP | 状態 | エラー |
|----------|------|--------|
| 192.168.125.19 | 接続不可 | Operation not permitted (22プロセス占有) |
| 192.168.125.13 | 接続不可 | No route to host |
| 192.168.125.45 | 接続不可 | UNREACHABLE |
| 192.168.125.62 | 接続不可 | プロセスハング |
| 192.168.125.12 | 接続不可 | 最古プロセス（4時間） |
| 192.168.125.79 | 応答遅延 | 1286ms ping |

---

## 8. 再発防止策

1. **コードレビュー**: 子プロセス起動時は必ずkill処理を実装
2. **監視強化**: ffmpegプロセス数のアラート設定
3. **テスト追加**: タイムアウト時のプロセス終了確認テスト
4. **定期クリーンアップ**: 5分以上生存するffmpegプロセスを自動kill

---

## 9. go2rtc統合の問題点

### 9.1 現在の実装

```
go2rtc優先条件: producersのrecv > 0（=視聴者がいる場合のみ）
```

**現在の状態**（障害発生時）:
- go2rtc登録ストリーム: 17件
- 視聴者あり: **0件**
- アクティブproducer: **0件**
- スナップショット取得元: **すべてffmpeg**

### 9.2 設計意図との乖離

**期待される動作**（ユーザー設計意図）:
1. サブネット巡回終了時にWSを切断
2. コンストラクト-ディストラクト型でクリーンなプロセス巡回
3. 再生中はセッション継続
4. 再生終了後は通常ループに戻る
5. クールダウン後に接続再確立して巡回

**実際の動作**:
1. go2rtcは**ライブビュー再生専用**として実装
2. 視聴者がいない通常のポーリングは**常にffmpeg**を使用
3. 巡回終了時のクリーンアップ処理**なし**
4. ffmpegプロセス管理の問題は**go2rtc統合後も残っている**

### 9.3 推奨される修正

1. **ポーリング時もgo2rtcのframe.jpeg APIを使用**
   - 視聴者がいなくてもgo2rtcにストリームを登録
   - frame.jpeg APIでスナップショット取得（ffmpeg不要）
   - go2rtcが内部でRTSP接続を管理

2. **サブネット巡回終了時のクリーンアップ**
   - 巡回完了後にgo2rtcからストリーム削除
   - 次の巡回開始時に再登録

---

## 10. 関連ファイル

- `src/snapshot_service/mod.rs` - スナップショット取得（要修正）
- `src/polling_orchestrator/mod.rs` - ポーリング制御
- `src/rtsp_manager/mod.rs` - RTSPアクセス制御
- `src/stream_gateway/mod.rs` - go2rtc統合

---

**報告者**: Claude Code
**ステータス**: 解析完了、修正待ち
