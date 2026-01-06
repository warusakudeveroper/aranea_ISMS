# is22 リファクタリング計画書

**作成日**: 2026-01-05
**対象システム**: is22 RTSPカメラ総合管理サーバー
**サーバー**: 192.168.125.246
**プロジェクトパス**: `code/orangePi/is22/`

---

## 1. 目的

**目標**: 全ファイルを400行以下に分割し、技術的負債を解消する

| 現状 | 目標 |
|------|------|
| routes.rs: 3006行 | 各ドメイン別ファイル 400行以下 |
| scanner.rs: 2296行 | 機能別ファイル 400行以下 |
| mod.rs (ipcam_scan): 1955行 | ステージ別ファイル 400行以下 |
| ScanModal.tsx: 1819行 | コンポーネント別ファイル 400行以下 |

---

## 2. 大原則確認

- **SSoT**: 各ファイルは単一の責務を持つ
- **SOLID-SRP**: 1ファイル1責務の徹底
- **MECE**: ファイル分割は重複なく網羅的に

---

## 3. 分割計画詳細

### 3.1 routes.rs (3006行) → 9ファイル

**現状構造**:
```
lines 1-21:    imports/use statements
lines 22-97:   Router definition (create_router)
lines 98-3006: Handler implementations
```

**分割計画**:

| 新ファイル | 行数目安 | 内容 |
|-----------|---------|------|
| `routes/mod.rs` | ~100 | create_router + re-exports |
| `routes/health.rs` | ~50 | health_check, device_status |
| `routes/cameras.rs` | ~400 | list_cameras, create_camera, get_camera, update_camera, delete_camera, camera_sort |
| `routes/modals.rs` | ~200 | modal_lease_*, modal_budget, active_modals |
| `routes/suggest.rs` | ~150 | get_suggest, suggest_context_*, get_presets |
| `routes/detection_logs.rs` | ~350 | detection_logs_*, performance_logs |
| `routes/ipcam_scan.rs` | ~400 | scan_*, device_*, approve_device |
| `routes/subnets.rs` | ~200 | subnet_*, credential_* |
| `routes/streams.rs` | ~150 | go2rtc_*, stream_gateway_* |

**依存関係**: `mod.rs` → 他全ファイル（並列実装可能）

---

### 3.2 scanner.rs (2296行) → 5ファイル

**現状構造**:
```
lines 1-28:      imports + PORT_WEIGHTS
lines 29-~1530:  OUI_CAMERA_VENDORS (巨大な静的データ)
lines ~1531-end: NetworkScanner impl + probe functions
```

**分割計画**:

| 新ファイル | 行数目安 | 内容 |
|-----------|---------|------|
| `scanner/mod.rs` | ~100 | re-exports, NetworkScanner struct definition |
| `scanner/oui_data.rs` | ~1500 | OUI_CAMERA_VENDORS (データファイルとして許容) |
| `scanner/port_weights.rs` | ~50 | PORT_WEIGHTS constant |
| `scanner/probes.rs` | ~350 | probe_rtsp, probe_onvif, probe_http |
| `scanner/network.rs` | ~300 | NetworkScanner impl, scan logic |

**注**: `oui_data.rs`は静的データのため400行超過を許容（コードではなくデータ）

**依存関係**:
1. `port_weights.rs` (依存なし)
2. `oui_data.rs` (依存なし)
3. `mod.rs` → port_weights, oui_data
4. `probes.rs` → mod.rs
5. `network.rs` → mod.rs, probes.rs

---

### 3.3 ipcam_scan/mod.rs (1955行) → 8ファイル

**現状構造**:
```
lines 1-150:     IpcamScan struct, job management
lines ~151-400:  Stage 0 (Discovery)
lines ~401-600:  Stage 1 (Port Scan)
lines ~601-800:  Stage 2 (RTSP Probe)
lines ~801-1000: Stage 3 (ONVIF Probe)
lines ~1001-1200: Stage 4 (HTTP Probe)
lines ~1201-1400: Stage 5 (Aggregation)
lines ~1401-1600: Stage 6 (Finalization)
lines ~1601-end: DB operations, helpers
```

**分割計画**:

| 新ファイル | 行数目安 | 内容 |
|-----------|---------|------|
| `ipcam_scan/mod.rs` | ~150 | re-exports, IpcamScan struct |
| `ipcam_scan/job.rs` | ~150 | ScanJob, job management |
| `ipcam_scan/stage0_discovery.rs` | ~200 | ARP/mDNS discovery |
| `ipcam_scan/stage1_portscan.rs` | ~200 | Port scanning |
| `ipcam_scan/stage2_rtsp.rs` | ~200 | RTSP probing |
| `ipcam_scan/stage3_onvif.rs` | ~200 | ONVIF probing |
| `ipcam_scan/stage4_http.rs` | ~200 | HTTP probing |
| `ipcam_scan/stage5_aggregate.rs` | ~200 | Result aggregation |
| `ipcam_scan/stage6_finalize.rs` | ~150 | Finalization |
| `ipcam_scan/db.rs` | ~300 | Database operations |

**依存関係**:
1. `job.rs` (依存なし)
2. `db.rs` (依存なし)
3. `mod.rs` → job.rs, db.rs
4. `stage0_discovery.rs` → mod.rs
5. `stage1_portscan.rs` → mod.rs, stage0
6. `stage2_rtsp.rs` → mod.rs, stage1
7. `stage3_onvif.rs` → mod.rs, stage2
8. `stage4_http.rs` → mod.rs, stage3
9. `stage5_aggregate.rs` → mod.rs, stage4
10. `stage6_finalize.rs` → mod.rs, stage5

---

### 3.4 ScanModal.tsx (1819行) → 10ファイル

**現状構造**:
```
lines 1-73:      LocalStorage cache utilities
lines 108-139:   RadarSpinner component
lines 141-393:   SubnetCredentialEditor (~252 lines)
lines 395-506:   ScanLogViewer (~111 lines)
lines 508-564:   PhaseIndicator (~56 lines)
lines 566-598:   DeviceStatusBadge (~32 lines)
lines 600-719:   DeviceCard (~119 lines)
lines 721-803:   categorizeDevice helpers (~82 lines)
lines 805-868:   CategorySection (~63 lines)
lines 870-1819:  Main ScanModal (~949 lines)
```

**分割計画**:

| 新ファイル | 行数目安 | 内容 |
|-----------|---------|------|
| `utils/scanCache.ts` | ~80 | LocalStorage cache utilities |
| `components/ui/RadarSpinner.tsx` | ~40 | Radar animation |
| `components/scan/SubnetCredentialEditor.tsx` | ~260 | Credential editor |
| `components/scan/ScanLogViewer.tsx` | ~120 | Log viewer |
| `components/scan/PhaseIndicator.tsx` | ~60 | Phase display |
| `components/scan/DeviceStatusBadge.tsx` | ~40 | Status badge |
| `components/scan/DeviceCard.tsx` | ~130 | Device card |
| `utils/deviceCategorization.ts` | ~90 | categorizeDevice helpers |
| `components/scan/CategorySection.tsx` | ~70 | Category section |
| `components/ScanModal.tsx` | ~400 | Main ScanModal (refactored) |

**依存関係**:
1. `scanCache.ts` (依存なし)
2. `deviceCategorization.ts` (依存なし)
3. `RadarSpinner.tsx` (依存なし)
4. `DeviceStatusBadge.tsx` (依存なし)
5. `PhaseIndicator.tsx` (依存なし)
6. `ScanLogViewer.tsx` (依存なし)
7. `DeviceCard.tsx` → DeviceStatusBadge
8. `CategorySection.tsx` → DeviceCard
9. `SubnetCredentialEditor.tsx` → scanCache
10. `ScanModal.tsx` → 上記全て

---

## 4. 実装順序

### Phase 1: 依存なしファイルの抽出 (並列実行可能)

| 優先度 | ファイル | 作業内容 |
|--------|---------|----------|
| 1-A | `scanner/port_weights.rs` | 定数抽出 |
| 1-B | `scanner/oui_data.rs` | OUIデータ抽出 |
| 1-C | `ipcam_scan/job.rs` | Job構造体抽出 |
| 1-D | `utils/scanCache.ts` | Cache utilities抽出 |
| 1-E | `utils/deviceCategorization.ts` | Helper functions抽出 |

### Phase 2: 基盤ファイルの分割

| 優先度 | ファイル | 作業内容 |
|--------|---------|----------|
| 2-A | `scanner/mod.rs` | Scanner struct定義 |
| 2-B | `scanner/probes.rs` | Probe関数抽出 |
| 2-C | `scanner/network.rs` | Network scan logic |
| 2-D | `ipcam_scan/db.rs` | DB operations抽出 |

### Phase 3: Stage分割 (順序依存あり)

| 優先度 | ファイル | 依存 |
|--------|---------|------|
| 3-A | `stage0_discovery.rs` | mod.rs |
| 3-B | `stage1_portscan.rs` | stage0 |
| 3-C | `stage2_rtsp.rs` | stage1 |
| 3-D | `stage3_onvif.rs` | stage2 |
| 3-E | `stage4_http.rs` | stage3 |
| 3-F | `stage5_aggregate.rs` | stage4 |
| 3-G | `stage6_finalize.rs` | stage5 |

### Phase 4: Routes分割 (並列実行可能)

| 優先度 | ファイル | 作業内容 |
|--------|---------|----------|
| 4-A | `routes/health.rs` | Health handlers |
| 4-B | `routes/cameras.rs` | Camera handlers |
| 4-C | `routes/modals.rs` | Modal handlers |
| 4-D | `routes/suggest.rs` | Suggest handlers |
| 4-E | `routes/detection_logs.rs` | Log handlers |
| 4-F | `routes/ipcam_scan.rs` | Scan handlers |
| 4-G | `routes/subnets.rs` | Subnet handlers |
| 4-H | `routes/streams.rs` | Stream handlers |
| 4-I | `routes/mod.rs` | Router定義 |

### Phase 5: Frontend分割

| 優先度 | ファイル | 依存 |
|--------|---------|------|
| 5-A | `RadarSpinner.tsx` | なし |
| 5-B | `DeviceStatusBadge.tsx` | なし |
| 5-C | `PhaseIndicator.tsx` | なし |
| 5-D | `ScanLogViewer.tsx` | なし |
| 5-E | `DeviceCard.tsx` | 5-B |
| 5-F | `CategorySection.tsx` | 5-E |
| 5-G | `SubnetCredentialEditor.tsx` | 1-D |
| 5-H | `ScanModal.tsx` | 上記全て |

---

## 5. テスト計画

### 5.1 ビルドテスト

| Phase | テスト項目 |
|-------|-----------|
| Phase 1完了後 | `cargo check` 成功確認 |
| Phase 2完了後 | `cargo build --release` 成功確認 |
| Phase 3完了後 | `cargo build --release` 成功確認 |
| Phase 4完了後 | `cargo build --release` 成功確認 |
| Phase 5完了後 | `npm run build` 成功確認 |

### 5.2 機能テスト

| テスト項目 | 確認方法 |
|-----------|----------|
| API全エンドポイント | curl/httpie で全API呼び出し |
| カメラスキャン | ScanModal UIで実行 |
| WebSocket | フロントエンド接続確認 |
| リアルタイム更新 | スナップショット更新確認 |

### 5.3 E2Eテスト

| テスト項目 | 確認方法 |
|-----------|----------|
| カメラ一覧表示 | ブラウザで確認 |
| スキャン実行 | ScanModal完全実行 |
| 巡回状態表示 | ヘッダー表示確認 |

---

## 6. MECE確認

| 観点 | 確認結果 |
|------|----------|
| ファイル分割 | ✅ 全大規模ファイル(4件)を網羅、重複なし |
| Backend | ✅ routes.rs, scanner.rs, mod.rs を網羅 |
| Frontend | ✅ ScanModal.tsx を網羅 |
| 依存関係 | ✅ 循環依存なし、実装順序明確 |
| テスト | ✅ ビルド/機能/E2E の3層で網羅 |

---

## 7. アンアンビギュアス確認

| 項目 | 状況 |
|------|------|
| 各ファイルの行数目安 | ✅ アンアンビギュアス - 具体的数値あり |
| 分割境界 | ✅ アンアンビギュアス - 関数/構造体単位で明確 |
| 実装順序 | ✅ アンアンビギュアス - 依存関係に基づく順序 |
| テスト項目 | ✅ アンアンビギュアス - 具体的確認方法あり |

---

## 8. リスクと対策

| リスク | 対策 |
|--------|------|
| 循環依存発生 | 依存グラフを事前確認、trait/interface で分離 |
| ビルドエラー | 各Phase後に必ずビルド確認 |
| 機能退行 | 分割前後でAPIレスポンス比較 |
| import漏れ | IDE補完とcargo clippy活用 |

---

## 9. 次のアクション

1. ✅ 本ドキュメント作成完了
2. ⏳ Phase 1 実装開始（依存なしファイル抽出）
3. ⏳ Phase 2-5 順次実装
4. ⏳ 全テスト実行
5. ⏳ Issue 3 (サブネット別巡回時間表示) 再開

---

## 10. 変更履歴

| 日付 | 変更内容 | 担当 |
|------|----------|------|
| 2026-01-05 | 初版作成 | Claude Code |
