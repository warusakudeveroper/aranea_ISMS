# Google Nest Doorbell (SDM) 詳細設計書（is22 Camserver / mAcT）

## 0. 本書の位置づけ
- 対象: is22 Camserver に Google Nest Doorbell を SDM（Device Access）経由で統合する実装の詳細設計。
- 参照: `GoogleDevices_introduction_BasicDesign.md`（全体方針）、`GoogleDevices_introduction_Environment .md`（コード外手順）。
- 範囲: バックエンド（Rust/Axum + go2rtc統合）、フロントエンド（React/shadcn 設定モーダル）、オペレーション/シークレット管理、テスト計画まで。デバイス実機設定やGCP/SDMコンソール操作は環境手順書に委譲。

### 本書で確定する前提（アンアンビギュアス）
- **SSoT**: SDM設定の正本は `settings.sdm_config`（DB）。`/etc/is22/secrets/sdm.env` は初期入力/バックアップ用保管庫であり、DB保存後は参照専用。UI保存はDBを必ず上書きし、差分を監査ログに残す。  
- **go2rtc要件**: v1.9.9+ を前提とし、Nest ソース文字列は `nest://{sdm_device_id}?project_id={project_id}&enterprise={enterprise_project_id}&client_id={client_id}&client_secret={client_secret}&refresh_token={refresh_token}` で生成し `/api/streams` へ登録する。秘密値は cameras テーブルに保存しない。  
- **RBAC/CSRF/監査**: `/api/sdm/*` は管理者ロールのみ許可し、Cookie運用時はCSRFトークン必須。全操作を監査ログに記録し、`client_secret`/`refresh_token` はマスクする。  
- **実装タイミング**: IpcamScan 側の修正完了後に実装する（プレースホルダ/ダミーボタンは追加しない）。

## 1. 目的・ゴール
- SDM経由で Nest Doorbell を is22 に登録し、既存 IpcamScan 由来カメラと同一UXで配信・静止画巡回・is21推論まで回す。
- 侵入最小（既存アーキに追従）、SSoT厳守（cameras/settings）、SOLID（sdm専用モジュール化）、アンアンビギュアスなAPI/UI定義を提供する。

## 2. SSoT / 責務境界（SOLID）
| 領域 | SSoT | 責務 |
|------|------|------|
| カメラ台帳 | `cameras` テーブル | Doorbell登録行（family=nest, discovery_method=sdm, sdm_device_id, snapshot_url 等） |
| SDM連携設定 | `settings` テーブル `sdm_config` | project/client/secrets/refresh_token/enterprise_id、状態フラグ |
| ストリーム | go2rtc (state) / ConfigStore (desired) | `camera_id` を source名とし、StreamGateway経由で登録・再同期 |
| 静止画 | SnapshotService | camera.snapshot_url（go2rtc frame.jpeg）経由で巡回取得 |
| フロント設定UI | React/shadcn `SystemSettingsModal` | SDM設定/認可/デバイス一覧/登録のプレゼンテーションのみ |

## 3. 前提・制約
- go2rtc が is22 と同居し HTTP API に到達できる（`GO2RTC_URL` 環境変数）。Nest入力が go2rtc でサポートされている前提。
- SDM API は OAuth2 refresh_token で長期運用する。token交換はバックエンドが担当。
- secrets は Git へ置かず、`/etc/is22/secrets/sdm.env` など OS 側に保持し、UI保存後に settings へ転記。
- IpcamScan では Doorbell を検出しない（仕様として明記済み）。SDM 登録導線を唯一経路とする。

## 4. 全体フロー（実装後の期待動作）
1. 管理者が設定モーダルで SDM 設定を入力 → `/api/sdm/config` へ保存。
2. 認可URL→code→refresh_token 交換（UI内 `exchange-code` or 事前取得を貼付）。
3. `/api/sdm/devices` で Doorbell 一覧を取得し、登録ダイアログで camera_id/fid/tid/location を指定。
4. バックエンドが `cameras` へ INSERT → StreamGateway で go2rtc に source追加 → snapshot_url を go2rtc frame に設定 → ConfigStore refresh。
5. PollingOrchestrator が HTTP snapshot 経由で巡回し is21 へ送信、UI は CameraGrid/Snapshot API で表示。
6. go2rtc 再起動時は reconcile ジョブが desired-state から再登録し、長時間運用を担保。

## 5. データ設計・マイグレーション
- `cameras` 追加カラム（最小構成）  
  - `sdm_device_id VARCHAR(128) NULL`（固定ID、SSoT）  
  - `sdm_structure VARCHAR(128) NULL`（任意：Home/Structure識別）  
  - `sdm_traits JSON NULL`（任意：traitsサマリを保存、AI hintsとは分離）
- マイグレーション例: `schema/010_sdm_integration.sql` を追加し、`CAMERA_COLUMNS` と `Camera` 構造体、更新bindへ反映（ConfigStore一貫性保持）。
- 既存列利用: `family="nest"`, `discovery_method="sdm"`, `snapshot_url=go2rtc frame`, `camera_context` は is21 hints 用に任意。
- 制約/インデックス:  
  - `sdm_device_id` にユニークインデックス（NULL許容、NOT NULL への移行は2段階で実施）。  
  - `camera_id` は英小文字/数字/ハイフンのみ、最大64文字（UI/BE共通バリデーション）。  
  - JSON列(`sdm_traits`) は1MB上限を設定し、必要なら要約を保存。  
  - 既存カラムを更新する場合は移行用スクリプトで既存行に `sdm_device_id=NULL` を明示し、ロールバック手順をDDLに付記。

## 6. 設定/シークレット構造（`sdm_config`）
- `settings` に JSON で保存するキー（例）:
```json
{
  "project_id": "my-sdm-project",
  "project_number": "123456789012",
  "enterprise_project_id": "enterprises/ABC",
  "client_id": "xxxxxxxx.apps.googleusercontent.com",
  "client_secret": "*****",
  "refresh_token": "*****",
  "last_synced_at": "ISO8601",
  "status": "configured|authorized|error",
  "error_reason": "optional"
}
```
- UI からは `client_secret` / `refresh_token` を再表示しない（保存のみ）。空文字は「更新なし」と解釈。
- secrets の正本は `settings.sdm_config`。`/etc/is22/secrets/sdm.env` は初期入力/バックアップ用の保管庫で、権限600を維持しつつ DB へ転記する。UI保存時は DB を上書きし、差分を監査ログに残す。

## 7. バックエンド詳細設計（Rust/Axum）
### 7.1 モジュール構成
- `sdm_integration`（新規）  
  - config: load/save/validate sdm_config（settings repository 経由、SSoTはDB）  
  - token: auth_code→refresh_token 交換、refresh_token→access_token 取得  
  - devices: SDM devices.list の呼び出しと正規化  
  - register: cameras INSERT + go2rtc source 追加 + snapshot_url 設定 + cache refresh  
  - reconcile: desired-state（ConfigStore）と go2rtc 実状態の差分適用（起動時+cron）

### 7.2 エンドポイント仕様（アンアンビギュアス）
| Method/Path | 入力 | 出力 | 失敗時 |
|-------------|------|------|--------|
| GET `/api/sdm/config` | なし | `{configured, project_id?, client_id?, has_refresh_token, status?, error_reason?}` | 500（settings取得失敗） |
| PUT `/api/sdm/config` | `{project_id, project_number?, enterprise_project_id, client_id, client_secret, refresh_token?}` | `{ok:true}` | 400（不足/不正）, 500（保存失敗） |
| POST `/api/sdm/exchange-code` | `{auth_code}` | `{ok:true, has_refresh_token:true}` | 400（code無効）, 500（Google疎通失敗） |
| GET `/api/sdm/devices` | なし（内部で access_token 取得） | `{devices:[{sdm_device_id,name,type,traits_summary,structure}]}` | 401（refresh_token無効）, 500 |
| POST `/api/sdm/devices/:id/register` | `{camera_id,name,location,fid,tid,camera_context?}` | Cameraオブジェクト | 400（重複/不足）, 409（camera_id衝突）, 500 |
| POST `/api/sdm/reconcile` | 任意（管理者向け手動トリガ） | `{added, skipped}` | 500 |
| GET `/api/sdm/test/go2rtc-version` | なし | `{version:"1.9.9", compatible:true}` | 500 |
| GET `/api/sdm/test/token` | なし | `{ok:true}` | 401/403/429/500 |
| GET `/api/sdm/test/device/:id` | なし | `{ok:true, traits_summary}` | 401/403/404/500 |

- 共通セキュリティ: `/api/sdm/*` は管理者ロール限定。Cookie運用時はCSRFトークン必須。全操作を監査ログへ記録し、`client_secret`/`refresh_token` はマスクする。`/api/sdm/devices` は quota 保護のため1分に1回まで。

- エラーログには client_secret / refresh_token を出さない（mask）。Google API 失敗は status と body を要約してログへ。

### 7.3 go2rtc 連携
- 前提バージョン: go2rtc v1.9.9+（nest ソース対応版）。  
- ソース文字列: `nest://{sdm_device_id}?project_id={project_id}&enterprise={enterprise_project_id}&client_id={client_id}&client_secret={client_secret}&refresh_token={refresh_token}`。StreamGateway が sdm_config を参照して組み立て、`POST /api/streams` に `{name: camera_id, source: <上記>}` を送る。  
- Reconcile:  
  - 起動時: ConfigStore に存在する Nest カメラを go2rtc `/api/streams` と突合し、欠損分を追加。  
  - 定期: 5分間隔で差分登録。  
  - 失敗は warn ログ＋メトリクス、次周期で再試行。  
- Secretsは go2rtc にのみ送信し、cameras 行には保存しない。

### 7.4 Snapshot/Polling 連携
- 登録時に `snapshot_url` を `http://<go2rtc>/api/streams/{camera_id}/frame.jpeg` に設定。解像度は go2rtc デフォルト（1280x720）を使用し、必要なら `frame?width=1280&height=720` を付与して固定する。  
- PollingOrchestrator は config refresh 後に Nest カメラも巡回対象に自動追加される想定。特別な並列数変更は不要（逐次）。  
- 取得間隔は既存カメラと同一設定を流用し、増加による帯域影響はメトリクスで監視する。

### 7.5 観測性
- logs: `sdm` target で info/warn/error を分類、token文字列は redact。  
- metrics: `sdm_devices_list_success_total`, `sdm_register_success_total`, `sdm_reconcile_added_total`, `sdm_reconcile_error_total` を追加検討。  
- healthcheck: `/api/status` に sdm_config 状態サマリ（configured/authorized/error）を含めることを検討。

### 7.6 トークンライフサイクルとリトライ
- 状態: `status` は `not_configured | configured_pending_auth | authorized | error`。`error_reason` に Google 401/403/429/5xx の要約を格納。  
- アクセストークン取得: `POST https://oauth2.googleapis.com/token` 失敗時は指数バックオフ（1s, 5s, 30s 最大3回）。3回失敗で `status=error` に更新し UI へ反映。  
- devices.list 401/403: refresh_token 失効とみなし `status=error` に即時更新、UIに「再認可」を表示。  
- quota超過(429): `status=error` にはしない。エラーカウンタを記録し10分待ちで再試行。  
- 監査: 認可コード交換・refresh_token更新時は監査ログに操作主体と結果のみを記録し、秘密値はマスク。

### 7.7 登録フローの整合性とロールバック
1. `POST /api/sdm/devices/:id/register` 受信後、`cameras` へ `enabled=false` でINSERT（family=nest, discovery_method=sdm, sdm_device_id保存）。同時にユニーク制約重複をチェック。  
2. go2rtcへ `add_source(camera_id, nest_source_string)` を実行。失敗時は DB 挿入をロールバックし 502 を返す（登録残骸を残さない）。  
3. go2rtc成功後に `snapshot_url` を frame.jpeg でUPDATEし `enabled=true` に変更。  
4. `config_store.refresh_cache()` を呼び、PollingOrchestratorが即時反映。  
5. 再登録/リトライは idempotent（同じ sdm_device_id に対する重複は 409 を返す）。

### 7.8 解除/削除フロー
- 新規エンドポイント（設計のみ）: `DELETE /api/sdm/devices/:id` で cameras 行を `enabled=false` にし、go2rtc の source を削除。`delete_source` 失敗時は warn ログを残し、次回 reconcile で再試行。  
- is21 ログ/スナップショットは残置（監査用）。必要に応じ retention 設定を別途適用。

### 7.9 高度な疎通テストAPI
- `/api/sdm/test/go2rtc-version`: go2rtc APIへアクセスしバージョンを返却。v1.9.9未満なら `compatible:false`。  
- `/api/sdm/test/token`: refresh_token から access_token を取得するテスト。401/403/429/5xx をHTTPステータスで返し、UIで色分けする。  
- `/api/sdm/test/device/:id`: SDM devices.get 相当で traits を取得し、認可/権限の有無を確認する。404 は未登録デバイスを示す。  
- いずれも管理者限定、監査ログ対象、秘密値はログに出さない。

## 8. フロントエンド詳細設計（React/shadcn）
### 8.1 エントリ
- `App.tsx` ヘッダ右上 Settings ボタンに `SystemSettingsModal` を紐付け。 Zustand/Query 既存ストアを再利用。

### 8.2 `SystemSettingsModal` 構成（Tabs）
1. **Google/Nest (SDM)**  
   - 状態表示: Not configured / Configured / Authorized / Error（`GET /api/sdm/config`）  
   - 設定フォーム: project_id, project_number, enterprise_project_id, client_id, client_secret, refresh_token（masked入力、空なら変更なし）  
   - 認可フロー: 認可URLを生成表示 → code貼付 → `/api/sdm/exchange-code`。既存refresh_token貼付も可。  
   - デバイス一覧: `/api/sdm/devices` 取得→table表示。  
   - 登録ダイアログ: camera_id自動提案（`nest-<device_id>`）, name/location/fid/tid/camera_context 入力 → `/api/sdm/devices/:id/register` 実行 → cameras refetch。  
   - エラー表示: 401時は「再認可」、go2rtc失敗時は「配信設定を確認」メッセージ。
2. Polling/Admission（将来拡張プレースホルダ、今回は非活性表示のみ）
3. その他（将来拡張プレースホルダ）

### 8.3 バリデーションとUX
- 必須: project_id, enterprise_project_id, client_id, client_secret。refresh_token は交換成功または貼付で取得。  
- camera_id は英小文字/数字/`-` のみ、既存ID重複を事前チェック（camerasクエリキャッシュ参照）。  
- 保存/登録時はローディング/トースト表示、成功時はモーダル維持（一覧即時更新）。

## 9. 運用・セキュリティ
- secrets は UI/API ログに出さない。`sdm_config` のマスク処理を一元化。  
- SDM設定の正本は `settings.sdm_config`。`/etc/is22/secrets/sdm.env` は初期入力/バックアップ用保管庫で、権限600を維持。  
- refresh_token 失効時は `status=error`＋`error_reason` に格納し、UIが明示。  
- go2rtc への通信は内部ネットワーク前提。認証が必要な場合は `GO2RTC_URL` に埋め込まず別途認証ヘッダを StreamGateway に実装。

## 10. 実装タスクリスト（MECE）
- T1: マイグレーション追加（cameras カラム）＋ ConfigStore 型/クエリ更新。
- T2: `sdm_integration` モジュール新設（config load/save/validate、token交換、devices.list）。
- T3: API ルーティング追加（/api/sdm/*）＋エラーハンドリング共通化（mask）。
- T4: StreamGateway 連携拡張（Nestソース生成・add_source）＋ reconcile ジョブ追加。
- T5: SnapshotService/PollingOrchestrator の HTTPフォールバック確認（snapshot_url設定を含む）。
- T6: フロント `SystemSettingsModal` 実装（Tabs/Forms/Devices table/登録ダイアログ、状態管理）。
- T7: UI エラーメッセージ・ローディング・再取得フロー実装。
- T8: ドキュメント/Issue 登録（本書と手順書のリンクを INDEX/Issue に反映）。

## 11. テスト計画（MECE・アンアンビギュアス）
- **バックエンド（Rust）**  
  - 単体: sdm_config バリデーション、token交換エラー分岐（401/403/429/5xx）と指数バックオフ、RBAC/CSRFエラー時の 401/403。  
  - 統合: `/api/sdm/config` PUT/GET、devices.list mock、register で cameras 挿入→go2rtc登録→refresh_cache が実行されること（go2rtc は stub）。go2rtc登録失敗時にDBロールバックされること。  
  - Reconcile: go2rtc 欠損検出で add_source が呼ばれること、失敗時にメトリクス/ログが増えること。  
  - 削除: `DELETE /api/sdm/devices/:id` で enabled=false と go2rtc delete が呼ばれ idempotent であること。  
- **フロントエンド（React）**  
  - コンポーネント: フォームバリデーション、状態表示（Not configured/Authorized/Error/ConfiguredPendingAuth）。  
  - API モック統合: devices table 表示、登録ダイアログ送信後の refetch、go2rtc未対応バージョン時の警告表示。  
- **Chrome実UI/UX（手動/E2E）**  
  - 設定モーダルを開き、config保存→code交換→devices取得→登録→CameraGrid反映までを通し操作。  
  - refresh_token失効（401）を模擬し Error 状態→再認可→復旧すること。  
  - go2rtc停止→reconcileで復旧することをブラウザ経由で確認（snapshot取得成功）。  
- **非機能**  
  - secrets マスク確認（ネットワークログ/ブラウザDevTools/サーバログに client_secret/refresh_token が出ない）。  
  - 長時間: go2rtc 再起動後に自動復旧するまでの時間測定（目標 <1周期）。  
  - レート制限: `/api/sdm/devices` の呼び出し間隔制御が効くこと。

## 12. リスク・未決
- go2rtc の Nest ソース対応がビルド/バージョンによって差異がある可能性。実機検証で文字列とバージョンを確定し、必要なら設定で上書きできるよう拡張する。  
- Google 側 UI/同意画面の変更により認可URLテンプレが変動する可能性。UI側のヘルプテキストは実際の Google 表記に依存しすぎないよう文言設計する。  
- Refresh token 失効頻度に応じて「再認可リマインダ」機構が必要になるかもしれない（将来検討）。

## 13. MECE / アンアンビギュアス確認
- 機能領域（データ/設定/バックエンド/フロント/運用/テスト）で重複なく網羅（MECE）し、各API・フローの入出力と失敗時挙動を明記（アンアンビギュアス）。  
- 残る不確定要素は go2rtc nest 対応版の実機検証に限られ、それ以外は実装手順に落とし込める粒度まで具体化できている。
