# IS22 カメラ登録テストスイート レビュー依頼

**日付**: 2026-01-20
**コミット**: 2d588f5
**対象**: カメラ登録フロー統合テスト

---

## 概要

IS22カメラサーバーのカメラ登録フローに対する統合テストスイートを実装しました。
テストはサーバー上で常時実行可能な状態で内蔵されています。

---

## 実装内容

### 1. カメラ登録機能の改善

#### approve_device (Category B登録)
- **manufacturer/model継承**: ipcamscan_devicesからcamerasテーブルへ継承
- **PTZ自動検出**: 登録時にONVIFプローブを実行し、ptz_supportedを自動設定
- **onvif_endpoint生成**: カメラファミリーからエンドポイントを自動生成

#### verify_device
- **ptz_supported抽出**: onvif_capabilities JSONからptz_supportを検出・設定

### 2. テストスイート構成

```
/opt/is22/tests/
├── README.md                     # テスト実行ガイド
├── run_all_tests.sh              # テストランナー
└── test_camera_registration.sh   # カメラ登録フローテスト
```

### 3. テスト内容

#### Phase 1: 接続検証
- API サーバー到達性
- MySQL 接続

#### Phase 2: 事前クリーンアップ
- 192.168.99.x テストデータの削除

#### Phase 3: テストデバイス注入
- Category B: verified + credentials success デバイス
- Category C: discovered デバイス

#### Phase 4: Category B 登録テスト
- approve_device API呼び出し
- 全フィールド検証

#### Phase 5: Category C 登録テスト (force_register)
- force_register=true で登録
- pending_auth ステータス検証
- 全フィールド検証

#### Phase 6: API経由検証
- /api/cameras からテストカメラ取得

#### Phase 7: 削除テスト
- DELETE /api/cameras/{id}

#### Phase 8: 完全クリーンアップ
- テストデータの完全削除
- 残留データ確認

---

## 検証フィールド一覧

| フィールド | 説明 | Category B | Category C |
|-----------|------|-----------|-----------|
| camera_id | カメラID | ✓ 生成 | ✓ 生成 |
| fid | 施設ID | ✓ 9999 | ✓ 9999 |
| family | カメラファミリー | ✓ tapo | ✓ unknown |
| access_family | アクセス制限 | ✓ unknown | ✓ unknown |
| mac_address | MACアドレス | ✓ AA:BB:CC:DD:EE:01 | ✓ AA:BB:CC:DD:EE:02 |
| manufacturer | メーカー | ✓ TestManufacturer | ✓ TestManufacturer |
| model | モデル | ✓ TestModel-B | ✓ TestModel-C |
| lacis_id | LacisID | ✓ 3022AABBCCDDEE01* | ✓ 3022AABBCCDDEE02* |
| onvif_endpoint | ONVIF EP | ✓ 自動生成 | - 未設定 |
| ptz_supported | PTZサポート | ✓ 0 (検出なし) | ✓ 0 |
| status | ステータス | active | ✓ pending_auth |
| polling_enabled | ポーリング | 1 | ✓ 0 |

---

## テスト実行結果

```
=============================================
TEST SUMMARY
=============================================
Tests run:    15
Tests passed: 32
Tests failed: 0
=============================================
ALL TESTS PASSED
```

**サーバー**: 192.168.125.246
**実行日時**: 2026-01-19 18:19 UTC

---

## テスト実行方法

### サーバー上で実行

```bash
cd /opt/is22/tests
./run_all_tests.sh
```

### リモートから実行

```bash
ssh mijeosadmin@192.168.125.246 "cd /opt/is22/tests && ./run_all_tests.sh"
```

---

## レビュー依頼項目

### 1. テストカバレッジの確認
- [ ] Category B / Category C の登録パターンは網羅されているか
- [ ] 検証フィールドに不足はないか
- [ ] エッジケース（重複登録、不正データ等）のテストは必要か

### 2. PTZ自動検出の仕様確認
- [ ] 登録時のONVIFプローブは適切か
- [ ] プローブ失敗時のデフォルト値（false）は妥当か
- [ ] 手動でのPTZ設定変更は引き続き可能か

### 3. フィールド継承の確認
- [ ] manufacturer/model の継承は approve_device / force_register_camera で一貫しているか
- [ ] access_family の自動設定ロジックは正しいか

### 4. テストデータの安全性
- [ ] 192.168.99.x レンジは本番環境と競合しないか
- [ ] fid=9999 は予約されているか

---

## 関連ファイル

- `code/orangePi/is22/src/ipcam_scan/mod.rs` - 登録ロジック変更
- `code/orangePi/is22/tests/test_camera_registration.sh` - テストスクリプト
- `code/orangePi/is22/tests/run_all_tests.sh` - テストランナー
- `code/orangePi/is22/tests/README.md` - ドキュメント

---

## 次のステップ（提案）

1. 追加テストケース検討
   - 重複登録時のエラーハンドリング
   - 不正なfidでの登録拒否
   - MACアドレス変更時の挙動

2. CI/CD統合
   - ビルド後の自動テスト実行
   - テスト失敗時の通知

3. パフォーマンステスト
   - 大量登録時の処理時間計測
   - 並列登録時の競合検証

---

**レビュー完了後は本ドキュメントに結果を追記してください。**
