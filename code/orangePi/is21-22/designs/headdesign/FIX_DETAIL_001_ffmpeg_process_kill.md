# FIX-001 詳細設計: ffmpegプロセスkill実装

**作成日**: 2026-01-05 11:15 JST
**親ドキュメント**: [FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md](./FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md)
**優先度**: P0（致命的）
**難易度**: 中

---

# 絶対ルール（大原則の宣言）

**以下を必ず復唱してから作業を開始すること**

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます。
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います。
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照の改変はゴールを動かすことにつながるため禁止です。
11. このドキュメントを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します。
12. ルール無視はルールを無視した時点からやり直すこと。

---

# 1. 修正概要

## 1.1 問題の要約

`snapshot_service/mod.rs`の`capture_rtsp()`関数において、`Command::output()`を使用しているため：
- tokio::time::timeoutでFutureがキャンセルされても子プロセス（ffmpeg）が終了しない
- 結果としてゾンビプロセスが蓄積し、カメラの接続スロットを枯渇させる

## 1.2 修正の意図

`Command::spawn()` + タイムアウト時の`child.kill()`パターンに変更し、タイムアウト発生時に子プロセスを確実に終了させる。

---

# 2. 影響範囲

## 2.1 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `src/snapshot_service/mod.rs` | `capture_rtsp()`関数の修正 |

## 2.2 依存関係

| 依存元 | 依存先 | 影響 |
|--------|--------|------|
| `polling_orchestrator/mod.rs` | `snapshot_service.capture()` | インターフェース変更なし |
| `SnapshotService::capture()` | `capture_rtsp()` | 内部実装変更のみ |

**結論**: 本修正はインターフェース変更を伴わないため、呼び出し元への影響はない。

---

# 3. 現状コード分析

## 3.1 問題箇所

**ファイル**: `src/snapshot_service/mod.rs` (Lines 270-303)

```rust
/// Capture frame from RTSP stream using ffmpeg
async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
    // ffmpeg command to capture 1 frame from RTSP and output as JPEG to stdout
    // -rtsp_transport tcp: Use TCP for RTSP (more reliable)
    // -frames:v 1: Capture only 1 frame
    // -f image2pipe -vcodec mjpeg: Output as MJPEG to pipe
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
        .output()  // ← 問題箇所
        .await
        .map_err(|e| Error::Internal(format!("ffmpeg execution failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Internal(format!(
            "ffmpeg failed: {}",
            stderr.trim()
        )));
    }

    if output.stdout.is_empty() {
        return Err(Error::Internal("ffmpeg returned empty output".to_string()));
    }

    Ok(output.stdout)
}
```

## 3.2 問題のMECE分析

| 側面 | 問題 | 影響 |
|------|------|------|
| **プロセス生成** | `Command::new()` は正常 | なし |
| **プロセス完了待機** | `output()` は完了を待つのみ | キャンセル時にプロセスが生存 |
| **タイムアウト処理** | 関数内に存在しない | 呼び出し元でのタイムアウトが無効化 |
| **プロセス終了** | 明示的なkill処理なし | ゾンビ蓄積 |

---

# 4. 修正設計

## 4.1 修正後コード

```rust
/// Capture frame from RTSP stream using ffmpeg
///
/// Uses spawn() + kill() pattern to ensure proper process cleanup on timeout
async fn capture_rtsp(&self, rtsp_url: &str) -> Result<Vec<u8>> {
    use std::process::Stdio;

    // Spawn ffmpeg process
    let mut child = Command::new("ffmpeg")
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Internal(format!("ffmpeg spawn failed: {}", e)))?;

    // Wait for completion with timeout
    let timeout_duration = Duration::from_secs(self.ffmpeg_timeout);

    match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            // Process completed within timeout
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Error::Internal(format!(
                    "ffmpeg failed: {}",
                    stderr.trim()
                )));
            }

            if output.stdout.is_empty() {
                return Err(Error::Internal("ffmpeg returned empty output".to_string()));
            }

            Ok(output.stdout)
        }
        Ok(Err(e)) => {
            // Process failed to complete
            Err(Error::Internal(format!("ffmpeg execution failed: {}", e)))
        }
        Err(_) => {
            // Timeout - kill the child process explicitly
            tracing::warn!(
                timeout_sec = self.ffmpeg_timeout,
                "ffmpeg timeout, killing child process"
            );

            // Kill the child process
            if let Err(e) = child.kill().await {
                tracing::error!(error = %e, "Failed to kill ffmpeg process");
            }

            // Wait for process to fully terminate
            let _ = child.wait().await;

            Err(Error::Internal(format!(
                "ffmpeg timeout ({}s)",
                self.ffmpeg_timeout
            )))
        }
    }
}
```

## 4.2 変更点のMECE分析

| 変更点 | 目的 | 効果 |
|--------|------|------|
| `output()` → `spawn()` | 子プロセスのハンドル取得 | kill可能になる |
| `Stdio::piped()` 追加 | stdout/stderrのキャプチャ | 出力取得を維持 |
| `tokio::time::timeout()` 追加 | 関数内でのタイムアウト管理 | 確実なタイムアウト処理 |
| `child.kill().await` 追加 | タイムアウト時のプロセス終了 | ゾンビ防止 |
| `child.wait().await` 追加 | プロセス完全終了の待機 | リソースクリーンアップ |

---

# 5. テスト計画

## 5.1 ユニットテスト

### テストケース1: 正常終了

| 項目 | 内容 |
|------|------|
| **入力** | 有効なRTSP URL（応答が速いカメラ） |
| **期待結果** | JPEG画像データが返却される |
| **確認項目** | - データサイズ > 0 - JPEGマジックナンバー確認（0xFFD8） |

### テストケース2: タイムアウト発生

| 項目 | 内容 |
|------|------|
| **入力** | 応答しないRTSP URL（無効なIP） |
| **期待結果** | タイムアウトエラーが返却される |
| **確認項目** | - エラーメッセージに"timeout"を含む - ffmpegプロセスが存在しないこと（`ps aux | grep ffmpeg`） |

### テストケース3: ffmpegエラー

| 項目 | 内容 |
|------|------|
| **入力** | 不正なRTSP URL（フォーマットエラー） |
| **期待結果** | ffmpegエラーが返却される |
| **確認項目** | - エラーメッセージにstderr内容を含む |

## 5.2 統合テスト

### テストケース4: ポーリングサイクルでのタイムアウト

| 項目 | 内容 |
|------|------|
| **入力** | 応答が遅いカメラ（192.168.125.19）を含むポーリング |
| **期待結果** | タイムアウト後、次のカメラに正常に移行 |
| **確認項目** | - ffmpegプロセス数が蓄積しないこと - 5分間のポーリング後、ffmpegプロセス数 < 5 |

## 5.3 Chromeでの確認

| 項目 | 確認内容 |
|------|----------|
| **CameraGrid表示** | カメラタイルが正常に更新されること |
| **SuggestPane** | タイムアウト発生時も他カメラの再生に影響しないこと |
| **EventLogPane** | タイムアウトエラーがログに表示されること |

---

# 6. 受け入れ基準

## 6.1 必須基準

| ID | 基準 | 検証方法 |
|----|------|----------|
| AC-001-1 | タイムアウト発生時にffmpegプロセスが終了すること | `ps aux | grep ffmpeg`で確認 |
| AC-001-2 | 正常終了時にJPEG画像が返却されること | テストケース1 |
| AC-001-3 | 5分間のポーリング後、ffmpegプロセス数 < 5 | 統合テスト |
| AC-001-4 | 既存のCameraGrid表示に影響しないこと | Chrome UI確認 |

## 6.2 確認コマンド

```bash
# ffmpegプロセス数確認
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "ps aux | grep ffmpeg | grep -v grep | wc -l"

# 5分間のプロセス監視
for i in {1..30}; do
  echo "$(date): $(ssh mijeosadmin@192.168.125.246 'ps aux | grep ffmpeg | grep -v grep | wc -l') processes"
  sleep 10
done
```

---

# 7. ロールバック計画

## 7.1 ロールバック条件

- 修正後に正常なカメラへの接続も失敗する場合
- ffmpegプロセスのkillに失敗してシステムが不安定になる場合

## 7.2 ロールバック手順

1. Gitで変更をリバート: `git revert <commit-hash>`
2. is22サービス再起動: `sudo systemctl restart is22`
3. 状態確認: `curl http://localhost:8080/api/cameras`

---

# 8. 実装チェックリスト

- [ ] 大原則を復唱した
- [ ] `capture_rtsp()`の修正が完了した
- [ ] コンパイルが通ることを確認した
- [ ] ユニットテストを実行した
- [ ] 統合テストを実行した
- [ ] Chromeで動作確認した
- [ ] ffmpegプロセス数が蓄積しないことを確認した
- [ ] 大原則を再度復唱した

---

# 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 11:15 | 初版作成 |

---

**作成者**: Claude Code
**ステータス**: 実装待ち
