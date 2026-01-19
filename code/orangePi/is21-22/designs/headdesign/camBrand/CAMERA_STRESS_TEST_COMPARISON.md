# カメラブランド別ストレステスト比較レポート

## テスト実施日
2026-01-17

## 目的
IS22のカメラブランド/シリーズ別アクセスインターバルガード実装のための事前調査

---

## 1. テスト対象

| ブランド | モデル | IP | 解像度 |
|----------|--------|-----|--------|
| TP-link VIGI | 不明 | 192.168.126.124 | 2304x1296 |
| TP-link Tapo | C200 | 192.168.125.45 | 1280x720 |
| NVT (JOOAN) | A6M-U | 192.168.77.23 | 2304x1296 |

---

## 2. テスト結果比較

### 2.1 RTSP再接続テスト

| テスト項目 | VIGI | Tapo | JOOAN (NVT) |
|-----------|------|------|-------------|
| 急速再接続（0秒） | ✅ **100%** (20/20) | ✅ **100%** (20/20) | ❌ **5%** (1/20) |
| 0.5秒間隔 | ✅ 100% (10/10) | ✅ 100% (10/10) | ⚠️ 80% (8/10) |
| 1秒間隔 | ✅ 100% (10/10) | ✅ 100% (10/10) | ⚠️ 70% (7/10) |
| 2秒間隔 | ✅ 100% (10/10) | ✅ 100% (10/10) | ✅ 100% (10/10) |

### 2.2 並列接続テスト

| テスト項目 | VIGI | Tapo | JOOAN (NVT) |
|-----------|------|------|-------------|
| 2並列 | ✅ **100%** | ❌ **0%** | 未計測 |
| 3並列 | ✅ **100%** | ❌ **11%** | ⚠️ 不安定 |
| 5並列 | ⚠️ 60% | ❌ **10%** | 未計測 |
| Main+Sub同時 | ✅ **100%** | ❌ **0%** | ⚠️ 50% |

### 2.3 長時間ストリーム（30秒）

| ブランド | 結果 | 備考 |
|----------|------|------|
| VIGI | ✅ 成功 | 安定 |
| Tapo | ⚠️ 成功 | DTS警告あり |
| JOOAN (NVT) | ✅ 成功 | 安定 |

### 2.4 HTTP/ONVIF

| ブランド | HTTP | ONVIF | ONVIFポート |
|----------|------|-------|-------------|
| VIGI | ✅ 100% | 要認証 | 2020 |
| Tapo | N/A | 要認証 | 2020 |
| JOOAN (NVT) | ✅ 100% | ✅ 100% | 8899 |

---

## 3. 総合評価

| 項目 | VIGI | Tapo | JOOAN (NVT) |
|------|------|------|-------------|
| 再接続安定性 | ★★★★★ | ★★★★★ | ★★☆☆☆ |
| 並列接続 | ★★★★☆ | ★☆☆☆☆ | ★★★☆☆ |
| 長時間安定性 | ★★★★★ | ★★★★☆ | ★★★★★ |
| Main/Sub同時 | ★★★★★ | ★☆☆☆☆ | ★★★☆☆ |
| ONVIF安定性 | ★★★☆☆ | ★★★☆☆ | ★★★★★ |
| **総合** | **4.75/5** | **2.75/5** | **3.25/5** |

---

## 4. ブランド別特性サマリー

### TP-link VIGI ✅ 最優秀
```
強み:
- 急速再接続OK（インターバル不要）
- 3並列まで安定
- Main/Sub同時取得可能

弱み:
- 5並列以上で不安定
- ONVIF認証必須

推奨設定:
- 最小再接続間隔: 0秒
- 最大並列接続: 3
```

### TP-link Tapo ⚠️ 制限あり
```
強み:
- シングル接続では非常に安定
- 急速再接続OK

弱み（致命的）:
- 並列接続に非対応（最大1接続）
- Main/Sub同時取得不可
- DTSタイムスタンプ不整合

推奨設定:
- 最小再接続間隔: 0秒
- 最大並列接続: 1 ★必須★
- go2rtcプロキシ推奨
```

### NVT (JOOAN) ⚠️ 再接続に注意
```
強み:
- ONVIF非常に安定（認証不要）
- 長時間ストリーム安定

弱み:
- 急速再接続に弱い（5%成功率）
- ストリーム切替に失敗しやすい

推奨設定:
- 最小再接続間隔: 2秒 ★必須★
- 接続維持を優先（頻繁な切断を避ける）
```

---

## 5. IS22アクセスインターバルガード設計案

### 5.1 ブランド別設定テーブル

```sql
CREATE TABLE camera_access_limits (
    family VARCHAR(32) PRIMARY KEY,
    min_reconnect_interval_ms INT NOT NULL DEFAULT 0,
    max_concurrent_connections INT NOT NULL DEFAULT 3,
    require_exclusive_lock BOOLEAN NOT NULL DEFAULT FALSE,
    notes TEXT
);

-- 推奨初期データ
INSERT INTO camera_access_limits VALUES
('vigi',  0,    3, FALSE, 'VIGI: 高性能、並列3まで安定'),
('tapo',  0,    1, TRUE,  'Tapo: 並列接続不可、排他ロック必須'),
('nvt',   2000, 2, FALSE, 'NVT(JOOAN): 2秒間隔必須');
```

### 5.2 アクセスガードロジック

```rust
pub struct CameraAccessGuard {
    family: CameraFamily,
    last_access: Instant,
    active_connections: AtomicU32,
}

impl CameraAccessGuard {
    pub async fn acquire(&self) -> Result<AccessToken> {
        let limits = self.get_limits();

        // 1. 並列接続数チェック
        if self.active_connections.load() >= limits.max_concurrent {
            if limits.require_exclusive_lock {
                // Tapo: キューで待機
                return self.wait_for_slot().await;
            }
            return Err(TooManyConnections);
        }

        // 2. 再接続インターバルチェック
        let elapsed = self.last_access.elapsed();
        if elapsed < limits.min_reconnect_interval {
            tokio::time::sleep(limits.min_reconnect_interval - elapsed).await;
        }

        self.active_connections.fetch_add(1);
        self.last_access = Instant::now();

        Ok(AccessToken::new(self))
    }
}
```

### 5.3 ファミリー判定

```rust
pub fn determine_access_policy(camera: &Camera) -> AccessPolicy {
    match camera.family.as_str() {
        "tapo" => AccessPolicy {
            min_interval_ms: 0,
            max_concurrent: 1,
            require_exclusive: true,
            use_proxy: true,  // go2rtc推奨
        },
        "vigi" => AccessPolicy {
            min_interval_ms: 0,
            max_concurrent: 3,
            require_exclusive: false,
            use_proxy: false,
        },
        "nvt" => AccessPolicy {
            min_interval_ms: 2000,
            max_concurrent: 2,
            require_exclusive: false,
            use_proxy: false,
        },
        _ => AccessPolicy::default(),  // 安全側に倒す
    }
}
```

---

## 6. 運用上の推奨事項

### 6.1 Tapoカメラ対策

1. **go2rtcプロキシ必須**
   - Tapoカメラは並列接続不可のためプロキシ経由を強く推奨
   - IS22 → go2rtc → Tapo の構成

2. **排他ロック実装**
   - go2rtcを使わない場合は接続排他ロック必須
   - 同一Tapoへの並列アクセスをキューイング

### 6.2 JOOANカメラ対策

1. **2秒インターバル必須**
   - スナップショット取得間隔を2秒以上に
   - ヘルスチェックも2秒以上の間隔

2. **接続維持優先**
   - 可能な限り接続を維持
   - 頻繁な接続切断を避ける

### 6.3 VIGIカメラ

1. **特別な対策不要**
   - 3並列までは制限なしで運用可能
   - 4台以上の同時ポーリングは制限

---

## 7. 結論

| ブランド | アクセスガード | 優先度 |
|----------|--------------|--------|
| **Tapo** | 並列制限(1)+排他ロック | ★★★★★ 必須 |
| **JOOAN (NVT)** | 2秒インターバル | ★★★★☆ 高 |
| **VIGI** | 並列制限(3) | ★★☆☆☆ 低 |

### 実装優先度

1. **P1（必須）**: Tapo並列制限 + 排他ロック
2. **P2（高）**: JOOAN 2秒インターバル
3. **P3（中）**: VIGI 3並列制限
4. **P4（低）**: デフォルトポリシー（安全側設定）

---

## 付録: ブランド判定フロー

```
カメラ登録時
    │
    ├─ ONVIFで取得 → Manufacturer
    │   ├─ "tp-link" → family = "tapo" or "vigi"
    │   │   └─ Model名で判定 ("Tapo" → tapo, "VIGI" → vigi)
    │   ├─ "NVT" → family = "nvt"
    │   └─ その他 → family = "unknown"
    │
    └─ OUIで補助判定
        └─ AltoBeam → camera_likely + nvt候補
```
