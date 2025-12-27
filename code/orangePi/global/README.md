# araneapi (OrangePi 共通モジュール)

Orange Pi / Armbian 向けの共通 Python モジュール群。  
`code/orangePi/is20s` などデバイス固有アプリから再利用する。

## 構成
- `araneapi/config.py` : 設定ロード（YAML/ENV）と dataclass 定義
- `araneapi/logging.py` : ロガー初期化
- `araneapi/lacis_id.py` : MAC から LacisID を生成
- `araneapi/device_register.py` : AraneaDeviceGate 登録（CIC取得）
- `araneapi/http_client.py` : HTTPクライアント共通ラッパー
- `araneapi/mqtt_client.py` : MQTT 接続・購読/発行ラッパー
- `araneapi/sqlite_store.py` : メモリリング + SQLite 永続化
- `araneapi/system_info.py` : uptime/メモリ等の取得
- `araneapi/tasks.py` : 周期タスク/キャンセル管理

## 利用方法
```
export PYTHONPATH=/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac\\ \\(3\\)/Documents/aranea_ISMS/code/orangePi

python - <<'PY'
from araneapi.config import load_config
cfg = load_config()
print(cfg)
PY
```

※ 実運用では `pip install -e code/orangePi/global` で開発インストールする想定。
