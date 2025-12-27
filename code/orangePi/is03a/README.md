# is03a (Orange Pi Zero3) - ローカル受信リレー & モニター

is01/02/04/05/06/10 等のローカル受信・冗長化・簡易操作を目的とした Armbian 向けサービス。HTTP/BLE ingest→リング/SQLite保存→Forwarder(将来再送)＋MQTTコマンド＋SSEライブモニターを提供。

## セットアップ（開発）
```
cd code/orangePi/is03a
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt
export PYTHONPATH=$(pwd)/..:$(pwd)/../global
uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload
```

## 主な機能
- BLEスキャン（Bleak）と HTTP ingest (`/api/ingest`) をリング/SQLiteに蓄積
- Forwarderキュー（primary→secondary、指数バックオフ、max_queue=5000）
- MQTTコマンド: `ping`/`relay_now`/`set_config`、60秒ごとに status publish
- UI `/` : ステータス・イベント表・ログ・設定編集・リアルタイムモニター (SSE)
- アクセス制御: allowed_sources で UI/設定/ログ/ingest を制限

## API
- `GET /api/status` : ring/queue/lastFlush/hostname/uptime/MQTT状態
- `GET /api/events?limit=N` : 最新イベント取得
- `GET /api/events/stream` : SSEによるライブイベント
- `POST /api/ingest` : is02→is03互換ペイロード
- `GET /api/logs?lines=200` : ログTail
- `GET/POST /api/config` : 設定取得/更新（config_override.yaml に保存）

## 設定
- 既定: `/etc/is03a/config.yaml`（ENV `IS03A_CONFIG` で上書き可）
- ENV上書き: `ARANEA_*` 系（tid/gate/relay/mqtt/allowed_sources 等）
- UIからの変更は `data_dir/config_override.yaml` に保存（ENVが最優先）

## systemd 実行
```
sudo cp is03a.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now is03a
```
