# FIX-003 詳細設計: 巡回終了時クリーンアップ実装

**作成日**: 2026-01-05 12:30 JST
**親ドキュメント**: [FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md](./FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md)
**優先度**: P2（改善）
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

`StreamGateway.remove_source()`が実装済みだが、どこからも呼び出されていない。結果として、go2rtcに登録されたストリームが永続的に残り、クリーンな状態が維持されない。

**設計意図**（ユーザー要望）:
- サブネット巡回終了時にWebSocket/RTSP接続を切断
- クールダウン期間中はクリーンな状態
- 次の巡回開始時に接続を再確立

**現状の動作**:
1. サブネット巡回開始時にgo2rtcにストリーム登録（`add_source`）
2. 巡回中はgo2rtc経由またはffmpeg経由でスナップショット取得
3. **巡回終了後もgo2rtcストリームは残る**（remove_sourceが呼ばれない）
4. 次サイクルでは既存ストリームが再利用される

## 1.2 修正の意図

サブネットサイクル完了時に`remove_source()`を呼び出し、コンストラクト-ディストラクト型のクリーンな巡回を実現する。

## 1.3 設計考慮事項

### 1.3.1 SuggestPane再生中のストリーム保護

SuggestPaneでカメラが再生中の場合、そのストリームを削除すると再生が中断される。
**対策**: 再生中カメラのストリームは削除しない（視聴者がいるストリームを保護）

### 1.3.2 go2rtcの内部管理

go2rtcは視聴者がいなくなると自動的にRTSP接続を切断する可能性がある。
**確認事項**: `remove_source()`がgo2rtc側でどのように処理されるか

---

# 2. 影響範囲

## 2.1 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `src/polling_orchestrator/mod.rs` | サイクル終了時クリーンアップ呼び出し追加 |
| `src/stream_gateway/mod.rs` | バッチ削除メソッド追加（オプション） |

## 2.2 依存関係

| 依存元 | 依存先 | 影響 |
|--------|--------|------|
| `PollingOrchestrator` | `StreamGateway` | 既存依存を活用 |
| `StreamGateway.remove_source()` | go2rtc API | 既存実装 |

**結論**: 既存の未使用メソッドを活用するため、新規インターフェースは不要。

---

# 3. 現状コード分析

## 3.1 未使用関数: StreamGateway.remove_source()

**ファイル**: `src/stream_gateway/mod.rs` (Lines 111-125)

```rust
/// Remove a stream source
pub async fn remove_source(&self, name: &str) -> Result<()> {
    let url = format!("{}/api/streams?name={}", self.base_url, name);

    let resp = self.client.delete(&url).send().await?;

    if !resp.status().is_success() {
        return Err(Error::Internal(format!(
            "Failed to remove stream source: {}",
            resp.status()
        )));
    }

    Ok(())
}
```

**状態**: 実装済み、呼び出し元なし

## 3.2 挿入位置: サブネットサイクル完了

**ファイル**: `src/polling_orchestrator/mod.rs` (Lines 652-667)

```rust
tracing::info!(
    subnet = %subnet,
    cycle = cycle_number,
    // ... other fields
    "Subnet cycle completed"
);

// Broadcast cycle stats to frontend (per subnet)
realtime_hub
    .broadcast(HubMessage::CycleStats(CycleStatsMessage {
        // ... fields
    }))
    .await;

// ← ここにクリーンアップを挿入
// Cooldown period (no countdown broadcast...)
```

## 3.3 go2rtcストリーム登録箇所

**ファイル**: `src/snapshot_service/mod.rs` or `src/polling_orchestrator/mod.rs`

確認が必要: ストリームがどこで登録されているか。登録されたストリーム名のリストを保持する必要がある。

---

# 4. 修正設計

## 4.1 修正方針

**選択肢A**: サイクル終了時にサブネット内全ストリームを削除（シンプル）
- サブネット内で巡回したカメラのストリームを全削除
- SuggestPane再生中のカメラは除外

**選択肢B**: go2rtcのストリームリストを取得し、視聴者なしのものを削除
- `list_streams()`で全ストリームを取得
- consumersが空のストリームを削除

**結論**: 選択肢Aを採用。サブネット内で処理したカメラを追跡する方がシンプル。

## 4.2 実装詳細

### 4.2.1 サブネットループ内でのカメラID追跡

```rust
// 巡回したカメラIDを記録
let mut processed_camera_ids: Vec<String> = Vec::new();

for (index, camera) in enabled.iter().enumerate() {
    // ... 既存処理 ...
    processed_camera_ids.push(camera.camera_id.clone());
}
```

### 4.2.2 サイクル終了時クリーンアップ

```rust
// After "Subnet cycle completed" log

// Cleanup: Remove go2rtc streams for cameras that are not being viewed
// Skip cameras that have active viewers (SuggestPane playback)
for camera_id in &processed_camera_ids {
    // Check if camera has active viewers
    let stream_name = format!("cam-{}", camera_id);

    // Query go2rtc to check for consumers
    let has_viewers = match stream_gateway.list_streams().await {
        Ok(streams) => {
            if let Some(obj) = streams.as_object() {
                if let Some(stream_info) = obj.get(&stream_name) {
                    // Check if consumers array is non-empty
                    stream_info.get("consumers")
                        .and_then(|c| c.as_array())
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !has_viewers {
        // Safe to remove - no active viewers
        if let Err(e) = stream_gateway.remove_source(&stream_name).await {
            tracing::warn!(
                camera_id = %camera_id,
                error = %e,
                "Failed to remove go2rtc stream"
            );
        } else {
            tracing::debug!(
                camera_id = %camera_id,
                "Removed go2rtc stream (no viewers)"
            );
        }
    } else {
        tracing::debug!(
            camera_id = %camera_id,
            "Keeping go2rtc stream (has viewers)"
        );
    }
}
```

### 4.2.3 効率化: バッチ削除ヘルパー（オプション）

`StreamGateway`にバッチ削除メソッドを追加:

```rust
/// Remove streams that have no consumers (batch cleanup)
pub async fn cleanup_idle_streams(&self) -> Result<u32> {
    let streams = self.list_streams().await?;
    let mut removed = 0u32;

    if let Some(obj) = streams.as_object() {
        for (name, info) in obj {
            // Check if no consumers
            let has_consumers = info.get("consumers")
                .and_then(|c| c.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if !has_consumers {
                if self.remove_source(name).await.is_ok() {
                    removed += 1;
                }
            }
        }
    }

    Ok(removed)
}
```

## 4.3 代替案: 最小限の変更

FIX-001（ffmpegプロセスkill）が実装されれば、ゾンビプロセス問題は解決する。
FIX-003は「あると望ましい」改善であり、FIX-001ほど緊急ではない。

**推奨**: FIX-001を先に実装し、FIX-003は状況を見て判断。

---

# 5. テスト計画

## 5.1 ユニットテスト

### テストケース1: 視聴者なしストリームの削除

| 項目 | 内容 |
|------|------|
| **前提条件** | go2rtcにストリームが登録済み、視聴者なし |
| **操作** | サブネットサイクル完了 |
| **期待結果** | ストリームが削除される |
| **確認項目** | `list_streams()`で該当ストリームが存在しない |

### テストケース2: 視聴者ありストリームの保護

| 項目 | 内容 |
|------|------|
| **前提条件** | go2rtcにストリームが登録済み、SuggestPaneで再生中 |
| **操作** | サブネットサイクル完了 |
| **期待結果** | ストリームは削除されない |
| **確認項目** | `list_streams()`で該当ストリームが存在する、SuggestPane再生が継続 |

### テストケース3: クールダウン後のストリーム状態

| 項目 | 内容 |
|------|------|
| **前提条件** | サイクル完了後、クールダウン期間中 |
| **操作** | go2rtcストリーム一覧を確認 |
| **期待結果** | 視聴者なしストリームが存在しない |
| **確認項目** | ストリーム数が視聴者ありカメラ数と一致 |

## 5.2 Chrome統合テスト

### テストケース4: SuggestPane再生中のサイクル完了

| 項目 | 内容 |
|------|------|
| **前提条件** | SuggestPaneでカメラ再生中 |
| **操作** | サブネットサイクル完了を待つ |
| **期待結果** | 再生が中断されない |
| **確認項目** | ビデオプレイヤーがエラーなく継続 |

---

# 6. 受け入れ基準

## 6.1 必須基準

| ID | 基準 | 検証方法 |
|----|------|----------|
| AC-003-1 | サイクル終了後に視聴者なしストリームが削除されること | テストケース1 |
| AC-003-2 | 視聴者ありストリームが保護されること | テストケース2 |
| AC-003-3 | SuggestPane再生中にサイクル完了しても再生が継続すること | テストケース4 |

## 6.2 確認コマンド

```bash
# go2rtcストリーム一覧確認
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "curl -s 'http://localhost:1984/api/streams' | python3 -c '
import json, sys
d = json.load(sys.stdin)
print(f\"Total streams: {len(d)}\")
for name, info in d.items():
    consumers = info.get(\"consumers\", []) or []
    print(f\"  {name[:40]}: {len(consumers)} consumers\")
'"

# サイクル完了後の確認（5分監視）
for i in {1..30}; do
  echo "$(date): $(curl -s 'http://192.168.125.246:1984/api/streams' | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')"
  sleep 10
done
```

---

# 7. ロールバック計画

## 7.1 ロールバック条件

- 修正後にSuggestPane再生が予期せず中断する場合
- go2rtc APIエラーが頻発する場合

## 7.2 ロールバック手順

1. Gitで変更をリバート: `git revert <commit-hash>`
2. is22サービス再起動: `sudo systemctl restart is22`
3. go2rtc再起動: `sudo systemctl restart go2rtc`

---

# 8. 実装優先度

## 8.1 依存関係

| 修正 | 依存先 | 理由 |
|------|--------|------|
| FIX-001 | なし | 独立して実装可能 |
| FIX-002 | なし | 独立して実装可能 |
| FIX-003 | FIX-001 | FIX-001がなければゾンビ問題は残る |

## 8.2 推奨実装順序

1. **FIX-001**（P0）: ffmpegプロセスkill - ゾンビ蓄積の根本解決
2. **FIX-002**（P1）: フロントエンド境界 - ユーザー体験改善
3. **FIX-003**（P2）: サイクル終了クリーンアップ - リソース効率改善

**FIX-003の条件付き実装**:
- FIX-001実装後にffmpegゾンビ問題が解消すれば、FIX-003は任意
- go2rtcのストリーム残留が問題になる場合のみ実装

---

# 9. 実装チェックリスト

- [ ] 大原則を復唱した
- [ ] FIX-001が実装済みであることを確認した
- [ ] `processed_camera_ids`追跡を追加した
- [ ] サイクル終了時クリーンアップを追加した
- [ ] 視聴者保護ロジックが動作することを確認した
- [ ] TypeScriptコンパイルが通ることを確認した
- [ ] テストケース1〜4を実行した
- [ ] Chrome統合テストを実行した
- [ ] 大原則を再度復唱した

---

# 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 12:30 | 初版作成 |

---

**作成者**: Claude Code
**ステータス**: 実装待ち（FIX-001完了後）
