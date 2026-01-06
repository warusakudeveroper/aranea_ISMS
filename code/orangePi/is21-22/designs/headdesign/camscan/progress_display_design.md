# 進捗表示改善 設計ドキュメント

## 1. 概要

### 1.1 目的
スキャン進捗の%表示を改善し、ユーザーに「止まっている？」という印象を与えないスムーズな進捗表示を実現する。

### 1.2 対象ファイル
- バックエンド: `src/ipcam_scan/mod.rs`
- フロントエンド: `frontend/src/components/CameraScanModal.tsx`

### 1.3 現状の問題点（Camscan_designers_review.md #7より）
- ステージごとにまとめて進捗が減算される
- 長時間同じ%のまま止まっているように見える

---

## 2. 設計

### 2.1 進捗計算の改善

#### 2.1.1 理論最大値の算定

```rust
struct ScanProgressCalculator {
    /// 各ステージの理論最大ホスト数
    stage_weights: Vec<StageWeight>,
    /// 総ポイント
    total_points: u32,
    /// 消費ポイント
    consumed_points: u32,
}

struct StageWeight {
    stage: ScanStage,
    max_hosts: u32,      // 理論最大値（/24 = 253）
    weight_per_host: u32, // 1ホストあたりのウェイト
}
```

#### 2.1.2 ステージ別ウェイト

| ステージ | 最大ホスト | ウェイト/ホスト | 最大ポイント |
|---------|-----------|----------------|-------------|
| ARP/Host Discovery | 253 | 1 | 253 |
| Port Scan | 253 | 2 | 506 |
| Protocol Probe | 50 (推定) | 3 | 150 |
| Auth Try | 20 (推定) | 5 | 100 |
| **合計** | - | - | **1009** |

#### 2.1.3 進捗更新タイミング

```rust
/// ホストごとに進捗更新
fn update_progress_per_host(&mut self, stage: ScanStage) {
    let weight = self.get_weight(stage);
    self.consumed_points += weight;

    // 100ミリ秒間隔でブロードキャスト（過負荷防止）
    if self.should_broadcast() {
        let percent = (self.consumed_points as f32 / self.total_points as f32) * 100.0;
        self.broadcast_progress(percent);
    }
}
```

### 2.2 ステージ終了時の調整

Brute Force Mode OFF時など、実行対象とならなかったホストは一括消費：

```rust
/// ステージ終了時に残りポイントを消費
fn complete_stage(&mut self, stage: ScanStage, actual_hosts: u32) {
    let max_hosts = self.get_max_hosts(stage);
    let skipped = max_hosts - actual_hosts;

    // スキップ分を一括消費（ガバッと減る）
    let weight = self.get_weight(stage);
    self.consumed_points += skipped * weight;

    self.broadcast_progress(self.calculate_percent());
}
```

### 2.3 UI表示

```
┌─────────────────────────────────────────────────┐
│ スキャン進捗                                     │
├─────────────────────────────────────────────────┤
│ ████████████░░░░░░░░░░░░░░░░░░░░░░ 42%          │
│                                                 │
│ [ステージ 2/4] ポートスキャン                   │
│ 処理中: 192.168.125.45                          │
│ 127 / 253 ホスト完了                            │
│                                                 │
│ 経過時間: 2分30秒                               │
│ 推定残り: 約4分                                  │
└─────────────────────────────────────────────────┘
```

---

## 3. テスト計画

### 3.1 単体テスト
1. 進捗計算の正確性テスト
2. ステージ完了時の調整テスト

### 3.2 UIテスト
1. 進捗バーのスムーズな更新確認
2. 推定残り時間の表示確認

---

## 4. MECE確認

- [x] 全ステージがウェイト定義されている
- [x] 進捗計算が100%を超えない設計
- [x] 更新頻度が適切に制御されている

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
