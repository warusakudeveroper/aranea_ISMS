# Repository Guidelines

### 大原則の宣言(必ず復唱必要)
1. SSoTの大原則を遵守すること
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます。
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います。
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報配下なる場合においても等価であり、優劣をつけて自己判断で不要と判断した情報を握り潰すような仕様としてはならない。全てのユーザーは情報を確認する権利を有する。ただし誤操作防止などの理由がある場合はこの限りではない。
6. "ファイルがあるからヨシ","見てないけど他の依存が大丈夫だからヨシ"、"とりあえず動かすのにハードコードで値取得したフリしてるけど動いてるからヨシ"といった日本でいうところの"現場猫案件"は禁止です
7. 優先順位は依存関係と実装順によってのみ定義されます。棚上げにして将来実装などというケースはありません。
8. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
9. チェック、テストのない完了報告は報告と見做されません。
10. 確証バイアスやサンクコストに絡め取られて観測された現象を否定してはなりません。
11. 使えない、仕様上の問題あるけど仕様だからOKといった問題解決を見ない矮小化は禁止します
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照の指定がある場合その改変はゴールを動かすことにつながるため禁止です。
11. 設計ドキュメントのない実装、タスクリスト、テストリストの無い受け入れ条件が曖昧な実装は禁止です。
12. ルール無視はルールを無視した時点からやり直すこと。
13. 報告は必ず日本語で行うこと
14. 何かのトラブルがあった際は外的要因や環境要因を疑う前に必ずコードを疑うこと
15. araneaDeviceGate,認証登録系の登録、実証とテストはaranea_SDKを使用する。

### あらゆるアクションに共通する基本の手順
1. 指示があった場合は必ず現象の確認もしくは現在の実装の確認を行う。
2. よほど簡易なタスクでない限りは必ず該当の設計、タスクに関する概要ドキュメントを作成する。
3. その後必ず概要ドキュメントと実体、実装の整合性が取れているか確認する。
4. 概要ドキュメントと実体、実装の生合成が確認できたら項目ごとの詳細かつアンアンビギュアス、MECEな詳細設計を作成する。
5. 設計インデックスとタスクリスト、テスト計画を作成する。テストは必ずフロントエンド、バックエンド、chromeアクセスでの実UI/UXのテストとし、テストの簡素化は原則として認められない。
6. これら全てが揃ったらGithubにIssue登録する。
7. その後実装前の段階を必ずコミットプッシュする。
8. 実装を開始する。必ず全フェースにおいて`大原則の宣言`の複勝を行う
9. テストまで実行し報告する


## Project Overview

araneaシリーズIoTデバイス開発(araneaDevices)プロジェクト。離島・VPN運用を前提とした温湿度監視・接点制御システム。ISMSシリーズは特定プロジェクトに帰属するプロトタイプの位置付けです。汎用一般ラインはaranea isシリーズです。


```
is01(BLE) → is03(表示) → is02(Wi-Fi中継) → クラウド(将来)
is04/is05 ↔ Zero3(MQTT/HTTP)
```
- 役割: is01 温湿度BLE, is02 ゲートウェイ, is03 Orange Pi Zero3, is04 接点出力, is05 8ch入力, is10 SSHポーラー, is20s Wiresharkアナライザー（テスト: 192.168.125.248/126.248/3.250:8080, SSH `mijeosadmin/mijeos12345@`), is21 画像推論サーバー (IP 192.168.3.240, user/root `mijeos12345@`), is22 RTSP管理サーバー (IP 192.168.125.246, サービス Apache/MariaDB/Samba/Mosquitto/SSH, ポート22/80/443/3306/139/445/3000-9999, 各種ユーザー/DB/Samba `mijeosadmin/mijeos12345@`).
- is21補足: Hostname `araneais21camimageEdgeAI`, systemd running, OS Armbian 25.11.2 noble, Kernel 6.1.115-vendor-rk35xx。
- is22補足: 開発環境 Node v18.19.1 / npm 9.2.0 / Yarn 1.22.22 / Python3 3.12.3 / Rust 1.92.0 / PHP 8.3.6、MariaDB root `mijeos12345@`・一般 `mijeosadmin/mijeos12345@`。
- LacisID形式: `[3][製品種別3桁][MAC12桁][製品コード4桁]` 例 `3001AABBCCDDEEFF0001`.
- テナント: 市山水産 tid `T2025120608261484221` fid `9000` (市山水産/牛深 `192.168.96.0/23`) メール `info+ichiyama@neki.tech` lacisID `12767487939173857894` CIC `263238`; mijeo.inc tid `T2025120621041161827` fid `0150`(HALE京都丹波口 `192.168.125.0/24`)/`0151`(東寺 `192.168.126.0/24`) メール `soejim@mijeos.com` lacisID `18217487937895888001` CIC `204965`.

## プロジェクト構成
- `docs/` 設計・解析資料、`araneaSDK/` SDK知見と設計リファレンス。
- `code/ESP32/` 各デバイスArduinoコード、`code/ESP32/global/` 共通ライブラリ（`library.properties` esp32）。
- `code/orangePi/` Armbian/FastAPI等（例: `is20s`）と `code/orangePi/global/`。各デバイス配下に `*.service` や `data/*.json`。
- `scripts/maintenance/` ローカルツール（`cleanup_settings.py`）。

## ビルド・実行・ツール
- ESP32: `cd code/ESP32/<device>` → Arduino IDE または `arduino-cli compile --fqbn esp32:esp32:<board> <device>.ino` → `arduino-cli upload -p <port> ...`。`../global` をライブラリ追加。Partitionは`min_spiffs`推奨（`huge_app`禁止）。
- is20s (FastAPI): `cd code/orangePi/is20s && python3 -m venv .venv && . .venv/bin/activate && pip install -r requirements.txt && export PYTHONPATH=$(pwd)/..:$(pwd)/../global && uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload`。
- is03 (Orange Pi Zero3): OSイメージ `Armbian_25.5.1_Orangepizero3_noble_current_6.12.23.img`。ユーザー `isms/isms12345@`。`sudo systemctl status is03-relay`、`curl http://localhost:8080/api/status` などで確認。
- is03データ保持: メモリ最新2000件、SQLite `/var/lib/is03-relay/relay.sqlite` にバッチ書き込み。テナント 市山水産株式会社 (TID/FID 上記)。
- ドメイン辞書検証（is20s）: `python tools/test_matching.py -t tools/test_cases.csv` または `-f <list>`。メンテ: `python3 scripts/maintenance/cleanup_settings.py --dry-run`。
- MCP: `mcp-arduino-esp32` で Arduino CLI を利用可能。

## ESP32開発注意
- 共通モジュール: lacisIDGenerator, araneaRegister, wifiManager, displayManager, ntpManager, settingManager(NVS/SPIFFS), HTTPManager, otaManager, Operator（is01は一部のみ、is02/04/05でフル活用）。
- ルーターis10: WAN側 ping/ポートスキャン無応答は正常。疎通確認はLAN内SSHのみ。ASUSWRTは遅いのでタイムアウト60秒以上。
- is01: I2Cは必ず直列実行。DeepSleep復帰→I2C初期化順序に注意。CIC未取得でもBLE広告は必須。
- デフォルトWiFi: SSID `cluster1`〜`cluster6`, pass `isms12345@`。Relay endpoint: primary `192.168.96.201`, secondary `192.168.96.202`。

## コーディング・命名
- ESP32 C++: 2スペース、同一行ブレース、CamelCaseクラス/UPPER_SNAKE定数（`AraneaSettings.*`）。共通は `global/`、固有設定は各デバイス配下。
- Python: 4スペース・型ヒント・f-string（`tools/test_matching.py`）。FastAPIは `app/` 配下で役割ごとに分割。テンプレ/データは環境非依存に。
- デバイスフォルダ/ファイルはスラッグ（`is05a`, `is20s` 等）で統一。

## テスト指針
- Pythonサービス: 上記辞書テスターと `GET /api/status` ローカル確認。解析・パース変更時は `tools/` に軽量スクリプトを追加。
- ESP32: ターゲットボードでコンパイルし、WiFi/MQTT/HTTPSを実機で確認。ネットワーク挙動変更時は `code/ESP32/is10m/design/evidence/` のスクリプトを流用。
- すべての報告でMECE/アンアンビギュアスかを明示し、テスト結果を添付する。

## コミット・PR
- 形式 `type(scope): summary`（例: `docs(is20s): update ingest plan`, `feat(is05a): add relay debounce`）。scopeはデバイスIDまたは `docs`/`scripts`/`global`。
- PR: what/why、影響デバイス/サービス、テスト証跡（コマンドや実機結果）、必要に応じUI/APIレスポンスのスクリーンショット。ドキュメント更新はコード変更と同期。

## セキュリティ・設定
- 生クレデンシャルやテナント固有IDのコミットは禁止（本ドキュメントの既知情報は参照用のみ）。秘密は `.env` や systemd override、テンプレートに保持し、`watch_config.json.template` などに留める。
- シリアルログや出力からも秘匿情報を除去して共有する。
