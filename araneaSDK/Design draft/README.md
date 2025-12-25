# AraneaSDK Design Draft（mobes2.0 連携計画）

mobes2.0 側と araneadevice 側が同じ手順・同じスキーマで実装できるようにするためのルールブック兼 SDK 設計草案。GitHub リポジトリは以下を正本として参照する。
- mobes2.0: https://github.com/warusakudeveroper/mobes2.0
- aranea_ISMS（本リポジトリ）: https://github.com/warusakudeveroper/aranea_ISMS

## イントロ
- 本ドキュメントは araneaSDK 設計草案の総覧です。詳細は項目別ドキュメントを参照してください。
- 項目別ドキュメント（すべて `araneaSDK/Design draft/` 配下）:
  - `01_authentication.md` … 認証二系統（ESP32 / Linux）と多言語対応
  - `02_register_endpoint.md` … araneaDeviceGate 登録フロー
  - `03_state_report.md` … deviceStateReport/ローカルレポートの実装仕様
  - `04_mqtt.md` … MQTT/WS トピック・テスト
  - `05_type_schema_registry.md` … Type 登録とスキーマ同期
  - `06_testing_and_monitoring.md` … 認証/HTTP/MQTT テストと Firestore/BQ 監視
  - `07_runtime_modules.md` … マルチランタイム共通モジュール + araneawebdevice
  - `08_document_management.md` … ドキュメント管理・メタデータ・AI連携

## 目的とスコープ（サマリ）
- レジスター（araneaDeviceGate）、状態レポート（deviceStateReport）、MQTT を統一ツールで登録・試験・検証。
- userObject / userObjectDetail の整合性、ProductType/ID とスキーマ互換性を mobes 側と同期。
- 既存 100 台超の archive_ISMS へ影響を出さず、SDK で検証→段階適用。
- lacisOath 認証を全系統共通で使用（パスワード廃止）。

## 2. 共有前提（ソース・スキーマ）
- エンドポイント現行既知:
  - Register: `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`
  - State Report: `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport`
- ID 仕様（固定）:
  - lacisId = `"3" + productType(3桁) + MAC12HEX + productCode(4桁)`（例: 301030C92212F6800001）
  - productCode: 現状 0096 固定。productType: 001(is01),002(is02),003(is03),004(is04),005(is05),006(is06),007(ar-cd07)…
- Firestore コレクション（mobes 側想定）:
  - userObject（デバイス抽象）、userObject_detail（MAC/型番詳細）、type registry（araneaDeviceConfig 等）。
- 外部資料: docs/common/araneaDeviceGate_spec.md, docs/common/system_integration.md（既存仕様を継承）。

## 3. AraneaSDK 構成案（最小ディレクトリ設計）
```
araneaSDK/
  config/            # エンドポイント/認証/tenant設定テンプレ
  cli/               # コマンド群（register/report/mqtt/schema/validate/test）
  lib/               # HTTP/MQTT client・スキーマ/認証ロジック
  fixtures/          # 型別サンプルpayload, schema snapshot
  docs/              # 運用手順/FAQ/リリースノート
  tests/             # e2e & integration test scripts
```
- 生成物（将来）：Python or Node CLI を想定。最初は HTTP/MQTT スクリプト + JSON テンプレで即運用できる形を優先。

## 4. 機能別設計方針
### 4.1 Register（araneaDeviceGate）
- lacisOath（lacisId/userId/CIC/method=register）を必須。tenant 主の認証のみ許可。
- 入力: userObject（lacisID, tid, typeDomain=araneaDevice, type=ar-XX）、deviceMeta（macAddress, productType, productCode）。
- 既存 MAC の型番変更時: 旧 userObject/_detail をハードデリート→新登録（docs/common/araneaDeviceGate_spec.md §2 の挙動）。
- 異なる tid / ordinaler での再登録時: CIC 無効化 + ownershipChanged フラグをレスポンスに含める（監査ログ）。
- SDK 実装:
  - `cli/register`: 単発登録・再登録・一括登録（CSV/JSON）を選択。レスポンスを log + Firestore 差分スナップショットとして保存。
  - `lib/register_client`: 3 回リトライ + Backoff。CIC 未保持時は自動 clearRegistration。

### 4.2 State Report / Telemetry
- 現行 HTTP endpoint（deviceStateReport）と将来 MQTT 双方に同一 payload を送る抽象化を用意。
- 必須フィールド例（productType=005 is05 など）:
  - state: { ch1..ch8 or sensor fields, lastUpdatedAt, rssi, ipaddr, SSID, firmwareVersion }
  - meta: { lacisId, tid, deviceName?, observedAt }
- SDK 実装:
  - `cli/report --type ar-is05 --payload fixtures/is05_state.json --endpoint http|mqtt`
  - スキーマ検証: mobes2.0 から取得した type schema で JSON Schema バリデーション。
  - 送信履歴は local JSONL に保存し、mobes 側 Firestore との diff チェックに利用。

### 4.3 MQTT 連携
- 想定トピック（案）: `aranea/{tid}/{lacisId}/state` (pub), `aranea/{tid}/{lacisId}/command` (sub)。
- 認証: mobes2.0 が提供する認証トークン（lacisOath 派生 or MQTT-specific credential）を想定。発行フロー整備後に SDK へ反映。
- SDK 実装:
  - `lib/mqtt_client`: TLS, LWT 設定、再接続、QoS 選択、スループット計測。
  - `cli/mqtt echo`: publish + subscribe echo 試験、payload スキーマ検証込み。

### 4.4 Type 登録・スキーマ管理（mobes 側で新設予定）
- mobes2.0 側 type registry（例: collection `araneaDeviceConfig`）と同期。araneaSDK は pull-first（読み込み）→差分提案→push（要承認）の流れ。
- 必須機能:
  - `cli/type fetch --domain araneaDevice` → 最新 schema / capabilities を fixtures に保存。
  - `cli/type diff --local fixtures/type/ar-is05.json --remote ...` → 破壊的変更を検出。
  - `cli/type register --file ...` → 新 type 登録（ProductType, ProductCode, MQTT topic, state schema, command schema, meta mapping）。
- バージョニング: `version`, `deprecated`, `supersedes` を schema に含め後方互換をチェック。

### 4.5 userObject / userObjectDetail ルール照合ツール
- 入力: Firestore dump（mobes 側）または API 経由取得の JSON。
- チェック内容:
  - lacisId 形式、productType/productCode 一致、MAC 一意性。
  - tid/ordinaler 不整合、cic_active=false デバイスの誤配信検出。
  - detail の meta（firmware, mqttConfig 等）と type schema の乖離。
- SDK 実装: `cli/validate user-object --dump dump.json --rules rulesets/aranea_device.yaml` で結果を HTML/CSV レポート化。

### 4.6 スキーマ取得・登録・拡張
- 取得: mobes2.0 type registry → `fixtures/type/` に保存（署名/ハッシュ付与して改ざん検出）。
- 拡張: device farm 導入時に必要な optional フィールドを proposal として PR 形式で出力。
- 互換性: breaking change は `breaking=true` フラグを強制し、SDK 側テストを全通過するまで本番に適用しない。

### 4.7 機能試験 / 通信試験
- 機能試験: 各 ProductType ごとに「登録→設定投入→状態送信→MQTT 応答」の最短シナリオを自動化。
- 通信試験: HTTP 3s timeout, MQTT 再接続, Wi-Fi 再接続を含む耐障害試験をスクリプト化。
- レポート形式: JSON + human-readable（md）。テスト結果は `tests/reports/yyyymmdd/` に保存。

### 4.8 ProductID / ProductType 管理
- 生成規則を SDK で共通関数化 (`lib/id.py`/`id.ts`)。
- テーブル管理: `fixtures/product_catalog.json` に type-domain, ProductType(3桁), ProductCode(4桁), 型番名, MQTT topic, state schema path。
- 一括登録: `cli/product sync --catalog fixtures/product_catalog.json` → mobes type registry に同期。

### 4.9 Device Farm 連携（既存 100 台超の扱い）
- archive_ISMS/ESP32 群は「監視専用」モードで接続し、SDK は read-only API で差分確認のみを先行。
- 新ファーム投入時は `--dry-run` で登録/設定/送信をシミュレートし、既存デバイスの CIC には触れない。
- デバイスごとに `device_profile.yaml` を置き、Wi-Fi/MQTT/HTTP の組み合わせ試験を自動生成。

### 4.10 AI モデルによる整合チェック（mobes 側提供を前提）
- API 化されたモデル（例: payload の正当性スコアリング）を `cli/ai-validate` から呼び出し、HTTP/MQTT 両経路のサンプルを評価。
- スコア < 閾値の場合はローカルでスキーマ乖離を差分提示し、mobes 側のフィードバック用 JSON を生成。

## 5. 運用同期プロセス
- ドキュメント同期: 本 Design Draft を mobes2.0 リポジトリにも置き、PR ベースで同期。差分が出た場合は必ず両リポジトリに同じ変更を反映。
- バージョン運用: SDK に `VERSION` ファイルを置き、mobes 側ツールと同一番号で進行。breaking change は `CHANGELOG.md` に必ず追記。
- 資格情報管理: lacisOath 資格と MQTT 資格は `.env.local`（git 管理外）に置き、cli 起動時に明示指定。

## 6. マイルストーン（案）
1) 調査フェーズ（~1w）: mobes2.0 type registry/API の現状確認、既存 Cloud Functions の挙動差分収集。
2) SDK スケルトン（~2w）: config/cli/lib/tests/fixtures の雛形と register/report/mqtt 最小コマンドを実装。is05 / is10 で通信確認。
3) ルール検証ツール（~1w）: userObject/detail バリデーション + product catalog 管理を実装。
4) Type 登録同期（~1w）: type fetch/diff/register のプロトタイピング。mobes 側と合意のうえ本番同期。
5) Device Farm サポート（~1w）: テストシナリオ自動生成 + 100 台監視モード投入。dry-run 済み後に一部デバイスで本番反映。

## 7. リスクと緩和策
- 既存デバイスの CIC/ownership 破壊リスク → `--dry-run` デフォルト、所有権変更時のみ明示フラグ `--force-ownership-change` を要求。
- スキーマ不一致 → mobes 側 schema を常に pull してから push。breaking change は両リポジトリに同時 PR。
- 認証情報漏洩 → `.env.local` + OS キーチェーン参照。ログには CIC を平文で残さない。
- MQTT 接続断 → 自動再接続と LWT 設定を必須化。オフライン時は HTTP fallback を選択。

## 8. 次のアクション（ドキュメント用 TODO）
- mobes2.0 側で Type 登録 API/コレクション仕様を確定し、SDK の `cli/type` 設計を固める。
- is05/is10 の実 payload から JSON Schema を抽出し、fixtures に初期セットを作成。
- 100 台監視モードの設計（何を read-only で見るか、どの頻度でポーリングするか）を詳細化。

## 9. 詳細設計: Type 登録・スキーマ同期ツール（概要）
### 9.1 役割と前提
- 目的: araneaDeviceGate 側（mobes2.0）に開発者が Type/Schema を登録し、aranea 側と同期した開発を行うための専用ツールセット。
- 利用者ロール: `mobes-admin`（書込可）, `aranea-dev`（読取/検証のみ）, `ci-bot`（PR 時の自動検証）。
- 環境: `dev`/`stg`/`prod` の 3 環境を前提。書込は dev→stg→prod の昇格のみ許可。

### 9.2 Type データモデル（Firestore 想定 araneaDeviceConfig）
```json
{
  "typeDomain": "araneaDevice",
  "productType": "005",
  "productCode": "0096",
  "typeId": "ar-is05",
  "displayName": "is05 8ch sensor",
  "stateSchema": { "$ref": "fixtures/type/ar-is05.state.schema.json" },
  "commandSchema": { "$ref": "fixtures/type/ar-is05.command.schema.json" },
  "mqtt": {
    "pubTopic": "aranea/{tid}/{lacisId}/state",
    "subTopic": "aranea/{tid}/{lacisId}/command",
    "qos": 1
  },
  "telemetry": {
    "requiredFields": ["state.ch1", "state.lastUpdatedAt"],
    "optionalFields": ["state.rssi", "state.ipaddr"],
    "maxReportIntervalSec": 90
  },
  "version": 3,
  "deprecated": false,
  "supersedes": ["ar-is05@2"],
  "hash": "sha256:xxxx",
  "updatedBy": "lacisId-of-editor",
  "updatedAt": "2025-01-21T12:00:00Z",
  "notes": "breaking change: added ch8"
}
```
- 必須キー: `typeDomain`, `productType`, `productCode`, `typeId`, `version`, `stateSchema`。
- バージョンは単調増加。breaking は別 version として登録し、旧版を `deprecated=true` で残す。

### 9.3 API/アクセス経路
- HTTP（将来 API）:
  - `GET /api/type?domain=araneaDevice` → 一覧
  - `GET /api/type/{typeId}` → 単体取得（ETag 付き）
  - `POST /api/type` → 新規登録（If-Match 空を許可）
  - `PUT /api/type/{typeId}` → 更新（If-Match=ETag 必須）
- Firestore 直接（暫定・管理者のみ）:
  - コレクション `araneaDeviceConfig` へ SDK からアクセス。`updateTime` で楽観ロック。
- 認証: lacisOath JWT（scope: type.read / type.write）。CI は SA キー + 短期トークン化。

### 9.4 CLI コマンド仕様
- `cli/type fetch --env dev --domain araneaDevice --out fixtures/type/`  
  最新一覧を取得し `.remote.json` で保存（ETag/hash を付与）。
- `cli/type diff --local fixtures/type/ar-is05.local.json --remote fixtures/type/ar-is05.remote.json`  
  state/command/mqtt/meta の差分と breaking の有無を表示。
- `cli/type register --file fixtures/type/ar-is05.local.json --env dev --force-breaking`  
  ETag チェックを通し dev に登録。breaking 変更はフラグ必須。
- `cli/type promote --type ar-is05 --from dev --to stg --allow-deprecated`  
  環境昇格（dev→stg→prod）。deprecated 昇格はフラグ必須。
- `cli/type lint --file fixtures/type/ar-is05.local.json`  
  JSON Schema の自己検証（$id/$schema/required/enum/format）。
- `cli/type lock --type ar-is05 --env prod --message "freeze for rollout"`  
  prod 更新を一時ロック。解除は `--unlock`。

### 9.5 開発フロー（同期手順）
1. `fetch`: dev/stg/prod の remote を取得。  
2. `edit`: `fixtures/type/ar-is05.local.json` を更新（state/command/mqtt/meta）。  
3. `lint`: スキーマ自己検証 + lacisId/productType/productCode 一貫性チェック。  
4. `diff`: remote と比較し breaking を確認。  
5. `register`: dev に登録（ETag 付き）。  
6. デバイスで schema 指定テスト（HTTP/MQTT 両方）。  
7. `promote`: stg→prod 昇格（CI で自動 diff/テスト通過が前提）。  
8. `lock`: リリース中は prod ロック、完了後アンロック。

### 9.6 バリデーション/ガード
- ETag（HTTP）または updateTime（Firestore）で楽観ロック。競合時は再 fetch → diff →再 apply。
- breaking（required 追加/enum 縮小/型変更）は `--force-breaking` がないと拒否。
- `productType`/`productCode` の再利用禁止（既存 type と lacisId プレフィクスを照合）。
- MQTT topic 衝突検出（同 topic に複数 type を割り当てない）。

### 9.7 アーティファクト管理
- `fixtures/type/{typeId}.state.schema.json` / `{typeId}.command.schema.json` を正本にする。
- `fixtures/type/{typeId}.local.json` は上記を参照するメタファイル（version/hash/mqtt/meta）。
- hash は `sha256(stateSchema + commandSchema + meta JSON canonical)` を計算し登録時に保存。
- CI で hash 変化が無ければ再登録をスキップし、誤更新を防ぐ。

### 9.8 テスト/CI
- `tests/type_validation.test.*`: 全 type の lint + fixtures と remote の一致確認。
- `tests/integration/register_and_report.sh`: dev で register→report→MQTT echo を実行。
- `tests/regression/breaking_guard.test.*`: `--force-breaking` 無しで失敗することを担保。

### 9.9 既存デバイスへのマイグレーション
- archive_ISMS 稼働デバイスは monitor-only で type 実態を収集し、差分をレポート。CIC や type を変更しない。
- 新 type を prod 昇格後、少数サンプルで再登録→状態送信→MQTT 応答を確認してから全体展開。
- 失敗時は `promote --to prev-env` 相当のロールバックで stg 安定版へ戻す。

## 10. 現行 araneadevice 実装との整合性（エンドポイント/ペイロード粒度）
### 10.1 実装済み Register フロー（共通モジュール: `code/ESP32/global/src/AraneaRegister.cpp`）
- デフォルト URL: `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate`（NVS 保存）。
- 送信 JSON（実装そのまま）:
```json
{
  "lacisOath": {
    "lacisId": "<tenantPrimary lacisId>",
    "userId": "<tenantPrimary email>",
    "cic": "<tenantPrimary cic>",
    "method": "register"
  },
  "userObject": {
    "lacisID": "<device lacisId>",
    "tid": "<tenantId>",
    "typeDomain": "araneaDevice",
    "type": "<deviceType e.g. ISMS_ar-is05a>"
  },
  "deviceMeta": {
    "macAddress": "<MAC12HEX>",
    "productType": "<3-digit>",
    "productCode": "<4-digit>"
  }
}
```
- 応答処理: HTTP 200/201 で `userObject.cic_code`, `stateEndpoint`, `mqttEndpoint?` を NVS 保存。409 は “Device already registered” で中断。JSON パース失敗時は再登録を要求。
- 認証: パスワードは廃止済み（lacisId + userId + CIC）。
- SDK での整合ポイント:
  - register CLI は上記フォーマットを正として生成/検証する。
  - ETag/ownershipChanged は将来拡張（現行レスポンスに含まれない場合は無視、含まれる場合は保存）。

### 10.2 実装済み State Report（例: is05a `code/ESP32/is05a/StateReporterIs05a.cpp`）
- ローカル送信（relay URL）payload:
```json
{
  "observedAt": "ISO8601",
  "sensor": { "lacisId": "<device>", "mac": "<MAC>", "productType": "005", "productCode": "0001" },
  "state": { "ch1": "OPEN/CLOSE", ..., "ch8_lastUpdatedAt": "timestamp", "rssi": "-60", "ipaddr": "192.168.0.x", "SSID": "..." },
  "meta": { "observedAt": "...", "direct": true },
  "gateway": { "lacisId": "<device>", "ip": "...", "rssi": -60 }
}
```
- クラウド送信（deviceStateReport）payload:
```json
{
  "auth": { "tid": "<tenantId>", "lacisId": "<device>", "cic": "<cic>" },
  "report": {
    "lacisId": "<device>",
    "type": "aranea_ar-is05a",
    "observedAt": "ISO8601",
    "state": { "ch1": "OPEN/CLOSE", ..., "ch8": "OPEN/CLOSE" }
  }
}
```
- 実装パラメータ: HTTP timeout 3000ms、最大連続失敗 3 回で 30 秒バックオフ、最小送信間隔 1s。Wi-Fi 未接続は即スキップ。
- SDK での整合ポイント:
  - fixtures に現行 payload を「実装正」として保存し、mobes 側 schema との差分を diff。
  - productCode は実装で `0001` 固定になっているため、仕様上 `0096` との乖離がある。Type 登録時にどちらを正とするか決定・移行計画を明記。
  - stateEndpoint は register 応答の値を優先。設定 UI から上書きされている場合もあるので CLI で参照・比較できるようにする。

### 10.3 既存 UI/設定項目（`HttpManager.cpp`）
- `/settings`: relay_pri/relay_sec（ローカル送信先）、cloud_url（クラウド送信先/deviceStateReport）、reboot_hour、wifi_retry_limit。
- `/tenant`: tid, tenant_lacisid, tenant_email, tenant_cic, gate_url（register 先）。
- `/wifi`: 最大6 SSID/PASS。HTTP パスワードは `http_pass`（デフォルト0000）。
- SDK での整合ポイント: CLI で config テンプレにこれらキーを持たせ、UI 設定値と齟齬がないか比較するコマンド `cli/config diff --device-dump ... --template ...` を追加予定。

### 10.4 差分・移行課題（要決定）
- productCode 既存実装の `0001` vs 仕様/カタログの `0096`。どちらを正にするかを決定し、register と state payload を同時に更新する移行手順を用意。
- type 名称: 実装は `aranea_ar-is05a` などを送信。Type registry での正式 `typeId` と揃える（例: `ar-is05`）。CLI の lint で不一致を検知。
- MQTT endpoint: register 応答に `mqttEndpoint` が無い場合の振る舞いを定義（現在は空保存）。Type registry で mqtt.topic を必須化し、ない場合は HTTP のみ運用と明記。
- 既存 NVS 登録済みデバイス: monitor-only で差分を採取し、CIC/endpoint を壊さずに type/schema のみ同期する手順を策定。

### 10.5 認証系の二系統（必須前提）
- 対象: ① ESP32 系（Arduino C++）と ② Linux SBC 系（OrangePi/Zero3 など, Python/Node 想定）。
- 共通仕様: lacisOath（lacisId + userId + CIC + method）を正本とし、パスワードは廃止。auth ペイロードは両実装で同一キー名を使用。
- SDK 提供物:
  - `lib/auth/arduino/`（または既存 `AraneaRegister` を内包）: ESP32 用に JSON 序列化と NVS 保持を共通化。
  - `lib/auth/python/` / `lib/auth/node/`: Linux SBC 用に同じ JSON を構築し、ETag/再試行/トークンキャッシュを実装。
- CLI は両系統の資格情報テンプレをサポート（`.env.esp32`, `.env.linux`）。登録・レポート・MQTT 試験コマンドはどちらの資格を使ったかをログに残す。

### 10.6 共通モジュールの多系統展開（auth/MQTT/HTTP 全般）
- ランタイム軸で共通部品を分け、同じプロトコル・同じスキーマを保持することが必須。
- 提供ターゲット（想定言語）:
  - ESP32: Arduino C++（既存 `AraneaRegister`, `StateReporter*` を SDK に内包）
  - Linux SBC: Python / Node.js / （将来）Rust
  - Web クライアント: araneawebdevice（ブラウザ/ServiceWorker 経由で MQTT over WS を想定）
- 共通 API サーフェス（各言語で揃える）:
  - `register(auth, userObject, deviceMeta)` → returns cic, stateEndpoint, mqttEndpoint
  - `report(auth, payload)` → HTTP/MQTT 送信、再試行・バックオフ含む
  - `mqtt.connect(endpoint, topics, opts)` → TLS/WS 対応、LWT、reconnect
  - `schema.validate(typeId, payload)` → JSON Schema バリデーション
- 配置案:
```
lib/
  arduino/   (auth, register, report, mqtt stub)
  python/    (auth, register, report, mqtt client)
  node/      (auth, register, report, mqtt client)
  rust/      (auth/report traits, mqtt client via rumqttc想定)   // PoC/将来枠
  web/       (auth/report via fetch, mqtt via WebSocket/MQTT.js)
```
- ビルド/配布:
  - Node: npm パッケージ `@aranea/sdk-node`
  - Python: PyPI `aranea-sdk`
  - Rust: crates.io（将来）、ESP32: ライブラリ同梱 or git submodule
- テスト統一: すべてのランタイムで fixtures の同一 JSON を入力し、署名付き hash が一致することを CI で確認（プロトコル同一性チェック）。

### 10.7 クラウド側現状と接続面
- 現稼働: Cloud Functions `araneaDeviceGate`（登録）と `deviceStateReport`（状態レポート）。MQTT ブローカーは未明示（要確認/将来追加）。Firestore で userObject/userObject_detail 管理。
- SDK との対応:
  - HTTP register/report は現行 CF を正本にし、エンドポイントは fixtures/config に保存。
  - MQTT は Type registry に topic/endpoint を持たせ、未稼働の場合は HTTP のみ運用と明記。
- araneawebdevice（Web 系）では MQTT over WebSocket を優先。CF が WS を返さない場合は fetch にフォールバック。

### 10.8 araneawebdevice（Web 実装）計画
- ユースケース: Web システム内で軽量な「仮想 araneaDevice」を実装し、mobes 側と同一スキーマで送受信・デバッグする。
- 構成:
  - `web/` パッケージで auth/report/mqtt をブラウザ対応（CORS/HTTPS 前提、認証は lacisOath JWT または短期トークン）。
  - ServiceWorker 経由でオフラインキャッシュし、再接続時にバッファを順次送信。
- 整合ポイント: ESP32 / Linux と同じ typeId/stateSchema/commandSchema を使用し、payload hash が一致するかを CI で比較。

### 10.9 開発効率を上げるテスト機能群（SDK + mobes 監視）
- 認証テスト:
  - `cli/auth test --env dev --lacisId ... --userId ... --cic ...` で register/report 両方の認証成否を即時確認。HTTP ステータスとエラー詳細を統一ログで出力。
  - バックオフ/再試行挙動を ESP32/Linux/Web で比較し、ハッシュ付きで差異検出。
- エンドポイントテスト（HTTP register/report）:
  - `cli/register test` でダミー登録→レスポンスと NVS/キャッシュ保存値を検証。
  - `cli/report test --payload fixtures/...` で deviceStateReport 送信し、応答コードとサーバ反映をチェック。
- MQTT テスト:
  - `cli/mqtt echo --env dev --topic aranea/{tid}/{lacisId}/state --payload ...` で pub/sub 往復を計測。WS/TLS/認証トークン共通化、LWT/再接続/throughput を測定。
- mobes 側の Firestore / BigQuery 監視:
  - テスト後に Firestore の userObject/detail/report 反映を `cli/report verify-firestore` で自動確認（差分・欠落検知）。
  - BigQuery 連携がある場合、直近インサートを `cli/report verify-bq --table ...` で確認し、遅延/欠落を検出。
  - CI では dev 環境で register→report→MQTT→Firestore/BQ 検証まで一連で実行し、両リポジトリに結果を共有。
