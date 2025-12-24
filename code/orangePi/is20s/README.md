# is20s (Orange Pi Zero3 / Armbian) - ローカルリレー & エッジゲート

Armbian 上で ESP 系デバイスのローカルリレー/エンドポイント投稿/MQTT 遠隔操作/簡易UIを提供するサービス。

## セットアップ（開発環境）
```
cd code/orangePi/is20s
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt

# PYTHONPATH を通す（araneapi を参照）
export PYTHONPATH=$(pwd)/..:$(pwd)/../global
uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload
```

## 設定
- 既定: `/etc/is20s/config.yaml`（ENV `IS20S_CONFIG` で上書き）
- ENV 例:
  - `ARANEA_TID`, `ARANEA_GATE_URL`, `ARANEA_RELAY_PRIMARY`, `ARANEA_RELAY_SECONDARY`
  - `ARANEA_MQTT_HOST`, `ARANEA_MQTT_PORT`, `ARANEA_MQTT_USER`, `ARANEA_MQTT_PASS`, `ARANEA_MQTT_TLS`
  - AraneaDeviceGate 登録用: `ARANEA_TENANT_LACISID`, `ARANEA_TENANT_EMAIL`, `ARANEA_TENANT_CIC`
  - アクセス制御: `ARANEA_ALLOWED_SOURCES`（カンマ区切り）
- UIからの設定変更は `data_dir/config_override.yaml` に保存され、起動時に自動マージ（ENVが最優先）。

## 実行/停止（systemd）
```
sudo cp is20s.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now is20s
```

## API / UI
- `GET /api/status` : ステータス（is03-relay互換キーを含む）
- `POST /api/ingest` : ローカルリレー受信（is02a→is03aペイロード互換、allowed_sourcesでIP制限可）
- `GET /api/events?limit=N` : イベント取得（リングバッファ）
- `GET/POST /api/config` : 設定取得/更新（UIと連動、config_override.yamlへ保存）
- `GET /api/logs?lines=200` : ログ参照（log_dir/is20s.log を読み出し）
- `POST /api/debug/publishSample` : デバッグイベント投入
- `/` : 統合UI（ステータス/イベント/ログ/設定編集、MQTT・Forwarder動的再起動対応）

## Forwarder・MQTT仕様
- Forwarder: primary→secondary順にPOST、失敗時は指数バックオフ（2s〜60s）、キュー上限5000件を超えると最古を破棄。
- MQTT: `ping`/`relay_now`/`set_config` を処理。`status`は60秒おきに `/status` トピックにハートビート送信。

## MQTT (初期トピック)
- subscribe: `aranea/{tid}/device/{lacisId}/command/#`
- publish:   `aranea/{tid}/device/{lacisId}/response`, `.../status`
