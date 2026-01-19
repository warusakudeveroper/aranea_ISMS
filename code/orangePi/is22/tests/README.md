# IS22 Integration Tests

IS22カメラサーバーの統合テストスイート。

## ディレクトリ構造

```
/opt/is22/tests/
├── README.md                         # このファイル
├── run_all_tests.sh                  # テストランナー
├── test_camera_registration.sh       # カメラ登録フローテスト
└── test_camera_registration_full.sh  # カメラ登録フルテスト(SSH経由)
```

## テスト実行方法

### サーバー上での実行（推奨）

```bash
cd /opt/is22/tests

# 全テスト実行
./run_all_tests.sh

# 特定のテストのみ実行
./run_all_tests.sh camera_registration

# 利用可能なテスト一覧
./run_all_tests.sh --list
```

### リモートからの実行

```bash
# SSH経由での実行
ssh mijeosadmin@192.168.125.246 "cd /opt/is22/tests && ./run_all_tests.sh"
```

## テストスイート一覧

### test_camera_registration.sh

カメラ登録フローの完全テスト。

**テスト内容:**
- Phase 1: 接続検証（API/MySQL）
- Phase 2: 事前クリーンアップ
- Phase 3: テストデバイス注入（ipcamscan_devices）
- Phase 4: Category B登録テスト（verified + credentials）
- Phase 5: Category C登録テスト（force_register）
- Phase 6: API経由でのカメラ確認
- Phase 7: API経由でのカメラ削除
- Phase 8: 完全クリーンアップ検証

**検証フィールド:**
- `camera_id`, `name`, `ip_address`
- `fid` - 施設ID
- `family` - カメラファミリー（tapo, vigi等）
- `access_family` - アクセス制限ファミリー
- `mac_address` - MACアドレス
- `manufacturer` - メーカー
- `model` - モデル
- `lacis_id` - MACベースのLacisID
- `onvif_endpoint` - ONVIFエンドポイント
- `ptz_supported` - PTZサポート（自動検出）

**テストデータ:**
- IPレンジ: `192.168.99.x`（本番と競合しない）
- FID: `9999`（テスト用）
- テスト完了後に完全削除

## 新規テストの追加方法

1. `test_<feature_name>.sh` ファイルを作成
2. 以下のテンプレートに従う:

```bash
#!/bin/bash
# =============================================================================
# <Feature Name> Integration Test
# =============================================================================

set -e

# ... テスト実装 ...

# 終了時に成功/失敗を返す
exit $TESTS_FAILED
```

3. `run_all_tests.sh` が自動的に `test_*.sh` を検出して実行

## CI/CD連携

```bash
# ビルド後にテスト実行
cargo build --release && ./tests/run_all_tests.sh

# 失敗時は非ゼロ終了コード
if ./tests/run_all_tests.sh; then
    echo "All tests passed"
else
    echo "Tests failed"
    exit 1
fi
```

## トラブルシューティング

### MySQL接続エラー

```bash
# パスワード確認
mysql -u root -p'mijeos12345@' camserver -e "SELECT 1"
```

### APIサーバー接続エラー

```bash
# サービス状態確認
sudo systemctl status is22

# ログ確認
sudo journalctl -u is22 -f
```

### テストデータが残った場合

```bash
# 手動クリーンアップ
mysql -u root -p'mijeos12345@' camserver -e "
DELETE FROM cameras WHERE ip_address LIKE '192.168.99.%';
DELETE FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';
"
```
