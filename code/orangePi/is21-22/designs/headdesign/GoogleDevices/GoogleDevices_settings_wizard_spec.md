# GoogleDevices 設定モーダル／SDMウィザード設計（ユーザー導線付き）

本書の目的: Google/Nest(SDM)連携を UI から迷わず完了させるため、設定モーダルの Google/Nest タブに「手順ガイド付きウィザード」を実装する詳細仕様を定義する。**実装着手はCamscan側修正完了後とし、プレースホルダ/ダミーUIは追加しない。** 本書ではモーダル単体で完結する UX の要件のみを先行確定する。

## 0. 位置づけと前提
- 対象: is22 Camserver の設定モーダルに新設する「Google/Nest (SDM)」タブ。
- SSoT: SDM設定は `settings.sdm_config`（DB）が正本。`/etc/is22/secrets/sdm.env` は初期投入用の運用メモ。ウィザードで保存した値は set_setting 経由で DB に反映し、再表示時はマスク済みサマリのみ返す。
- go2rtc: v1.9.9+ 前提。StreamGateway が `nest://{sdm_device_id}?project_id=...&enterprise=...&client_id=...&client_secret=...&refresh_token=...` を生成し `/api/streams` へ登録する。未満バージョンを検出した場合はウィザード内で警告を表示し、登録をブロックする。
- 範囲: UI フロー・表示文言・API 呼び出し・エラーハンドリング・完了確認まで。Camscan との連携ボタンは「後から有効化するプレースホルダ」として仕様化のみする。

## UX/ナビゲーション（リテラシ低め前提）
- 全ステップを「次へ」「戻る」で移動可能。進捗インジケータ（1/6など）を常時表示。  
- 各ステップは情報を分割し、必須入力が完了するまで「次へ」を非活性。  
- 完了したステップにはチェックマークを表示し「できた」を視覚化。  
- エラー時はステップ内に赤枠＋簡潔メッセージ（1行）で出し、スクロールせずに読める位置に表示。  
- サマリ/ヘルプは折りたたみで必要なときだけ展開（情報過多を防ぐ）。  
- ブラウザ戻るに依存せず、ウィザード内の「戻る」ボタンで前ステップに戻れる。

## 1. 状態モデル（アンアンビギュアス）
`GET /api/sdm/config` の返却を以下の UI 状態にマップする。
- `NotConfigured`: project_id/client_id 未登録。ボタン: 「はじめる」。
- `ConfiguredPendingAuth`: client_secret 保存済みだが refresh_token なし。ボタン: 「認可URLを開く」「認可コードを貼り付ける」。
- `Authorized`: refresh_token 有効。ボタン: 「デバイス一覧を取得」「登録ウィザードへ」。
- `Error`: `status=error` かつ `error_reason` あり。ボタン: 「再認可する」。エラー文言を表示。

## 2. ウィザード構成（ステップと画面要素）
各ステップはタブ内に段階表示し、進捗バーを表示する。
1) **概要・必要なもの（フールプルーフ前提チェック）**  
   - 何が必要かを箇条書き（project_id, client_id/secret, enterprise_project_id, refresh_token）。  
   - アカウント要件: 個人Gmail推奨、Workspaceは失敗する場合あり。  
   - Device Access登録に**$5課金が必要**で、支払い後に有効化まで数分かかることを赤字で表示。  
   - Home確認: DoorbellがGoogle Homeに登録済みか、複数Homeがある場合は正しいHomeを選択する。  
   - 「準備手順を見る」ボタンで Environment 手順書 (GD-02) の要約をモーダル内に展開（外部検索不要）。  
   - 「次へ」ボタンはチェックボックス（上記4点の自己確認）がすべてONになるまで押せない。
2) **SDM設定入力**  
   - フォーム: project_id, project_number(任意), enterprise_project_id, client_id, client_secret, refresh_token(任意)。  
   - 保存ボタン → `PUT /api/sdm/config`。空文字は「変更なし」と扱う旨を注記。  
   - 保存後は状態をリフレッシュし、成功時は「保存しました ✓」のフィードバックを表示。「戻る」でStep1に戻れる。
3) **認可コード取得**（ConfiguredPendingAuth のみ表示）  
   - 認可URLを**自動生成**して表示（コピー/「ブラウザで開く」ボタン付き）。  
   - ガイド: ブラウザで許可 → `https://www.google.com/?code=XXXX` にリダイレクト → `code=` 以降をコピーして貼り付けるスクリーンショット付き説明。  
   - 認可コード入力欄 + 「コードを交換」ボタン → `POST /api/sdm/exchange-code`。  
   - 失敗時のメッセージ:  
     - 401/invalid_grant: 「コードが古いか無効です。もう一度URLを開いてやり直してください」  
     - redirect_uri mismatch: 「Redirect URI が違います。`https://www.google.com` を使ってください」
4) **デバイス一覧取得**（Authorized）  
   - 「デバイスを取得」ボタン → `GET /api/sdm/devices`。  
   - 0件だった場合のチェックリスト（Yes/Noで潰す形式を同画面に表示）:  
     1. 同じGoogleアカウントでHomeにDoorbellを登録していますか？  
     2. Device Access ConsoleでOAuth Clientを紐付けましたか？  
     3. 認可URLでHome/デバイスへのアクセスを許可しましたか？  
     4. 複数Homeがある場合、正しいHomeを選択しましたか？  
     5. $5課金を完了し、数分待ちましたか？  
   - テーブル: name/type/structure/traits_summary/sdm_device_id。  
   - 行ごとに「登録する」ボタン。
5) **登録ダイアログ**  
   - フィールド: camera_id(自動提案: `nest-<短縮device_id>`), name, location, fid, tid(必須), camera_context(任意), snapshot_interval(任意)。  
   - 送信 → `POST /api/sdm/devices/:id/register`。成功で go2rtc 追加・config refresh。  
   - 失敗時の分岐を明記: 401→「再認可」、409→「camera_id 重複」、go2rtc失敗→「配信設定を確認」。  
   - 成功時はモーダルに「登録しました ✓」を表示し、自動でStep6へ遷移。  
6) **接続確認（つながった!!）**  
   - `GET /api/streams/{camera_id}/snapshot` を叩きプレビュー表示。  
   - 成功: 緑のバッジ「つながりました」。  
   - 失敗: 赤バッジと再試行ボタン、 go2rtc URL 設定へのリンク。  
   - 「is21に送るテスト」ボタン（任意）: snapshot を is21 に送信しレスポンスを表示。  
   - 「完了」ボタンでウィザードを終了し、必要なら「最初に戻る」を表示。

## 3. UI詳細仕様
- タブ名: `Google/Nest (SDM)`。ヘッダ右上 Settings からモーダルを開く。  
- ヘルプパネル: 右カラムに「よくある詰まり」と対処（refresh_token失効、認可URLエラー、デバイスに見えない場合）。  
- **高度な設定（折りたたみ）**:  
  - go2rtcバージョンチェックボタン → `/api/sdm/test/go2rtc-version` を叩き、v1.9.9未満なら赤バッジと更新手順リンクを表示。  
  - SDMアクセストークン取得テスト → `/api/sdm/test/token` で200/401/429を色分け表示。  
  - 対象デバイスでの疎通テスト → デバイス選択後に `/api/sdm/test/device/:id` を叩き、traits取得可否とステータスを表示。  
  - Snapshotプレビュー再取得ボタン（登録後専用） → `/api/streams/{camera_id}/snapshot` を叩き結果を表示。
  - go2rtc更新ガイド: バージョンが未満の場合に表示する手順を明記（例: `curl http://localhost:1984/api/version` で確認 → バイナリ更新/再起動の運用手順リンク）。
- 文言例（抜粋）:
  - ステップ1: 「Google Home で Doorbell をセットアップ済みか確認し、project_id/client_id を用意してください。」  
  - ステップ3: 「表示されたURLを開き、Googleアカウントで許可し、?code= 以降の値を貼り付けてください。」  
  - ステップ6: 「プレビューが表示されれば登録完了です。プレビューが出ない場合は再認可または go2rtc の状態を確認してください。」
- バリデーション: project_id/client_id/enterprise_project_id は必須。camera_id は英小文字・数字・ハイフンのみ、最大64文字。fid/tid 必須（サブネット整理に準拠）。
- マスク: `client_secret`/`refresh_token` は再表示しない。UI上は「保存済み」のバッジのみ表示。

## 4. エラーハンドリングとリカバリ
- Google API 401/403: `status=error` にし、UIで「再認可が必要」と明示。  
- go2rtc 接続失敗: 登録は成功扱いとしつつ「配信未設定」警告を出し、再同期（reconcile）で復旧する旨を表示。  
- device.list で対象が出ない: 上記チェックリストを表示し、Yes/Noで潰しながら再実行ボタンを案内。  
- ネットワーク失敗: リトライボタンとバックオフのガイダンスを表示。  
- UIタイムアウト: 10秒超で「遅延しています、再実行してください」と表示。

### よくある詰まり専用メッセージ（ヘルプパネルに常時表示）
- 「課金していない/有効化待ちです」→ Device Access Console で$5支払い後、数分待って再実行。  
- 「Workspaceアカウントで失敗」→ 個人Gmailで試してください。  
- 「Homeにデバイスが見えない」→ HomeアプリでDoorbellが登録済みか確認し、正しいHomeを選択。  
- 「複数Homeがある」→ 認可URLを開いたときに表示されるHomeを確認し、目的のHomeを選択。  
- 「redirect_uriエラー」→ 認可URLで `redirect_uri=https://www.google.com` になっているか確認。  
- 「デバイスが0件」→ 上記チェックリストを順に潰し、再実行。  

## 5. 実装タイミングとCamscan連携
- 本ウィザードの実装は **Camscan 側の修正完了後に着手** し、プレースホルダや無効ボタンは作らない。  
- Camscan 側で `suggested_action=SdmRegister` を出す修正が完了した時点で、Scan結果から本タブを開く導線を実装する（その時点で別途Issue/タスク化する）。

## 6. テスト計画（本ウィザード専用）
- 単体/モック:  
  - `/api/sdm/config` 状態別の UI 遷移 (NotConfigured/ConfiguredPendingAuth/Authorized/Error)。  
  - 認可URL生成・code 交換成功/失敗の分岐表示。  
  - デバイス一覧テーブルの表示と空リスト表示。  
  - 登録フォームのバリデーション（必須/形式/重複camera_id）。  
  - go2rtc エラー時の警告表示。
- 統合/E2E (Chrome):  
  - ステップ1→6 を通しで操作し、プレビュー表示まで確認。  
  - refresh_token 無効化→再認可で復旧する流れ。  
  - SDM未設定状態で「デバイスを取得」を押した場合のエラーメッセージ。  
  - (後日) Camscan からの導線でタブが開くこと。

## 7. 既知の未決/残課題
- go2rtc Nest ソース文字列の具体値・対応バージョンは別途確定が必要（GDレビュー指摘継続）。  
- `/api/sdm/*` の RBAC/監査ログ要件は別途セキュリティ設計に従う（現状は管理者限定を想定）。  
- reconcile 周期・メトリクス設計は GoogleDevices_review の残項目として継続管理。
