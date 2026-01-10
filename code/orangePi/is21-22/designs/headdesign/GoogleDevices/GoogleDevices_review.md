# GoogleDevices SDM/Nest Doorbell 設計レビュー（The_golden_rules準拠）

対象ドキュメント: `code/orangePi/is21-22/designs/headdesign/GoogleDevices/ GoogleDevices_introduction_BasicDesign.md`, `code/orangePi/is21-22/designs/headdesign/GoogleDevices/GoogleDevices_introduction_DetailedDesign.md`, `code/orangePi/is21-22/designs/headdesign/GoogleDevices/ GoogleDevices_introduction_Environment .md`

## 確認結果（充足）
- SSoT統一: `settings.sdm_config` を正本、`/etc/is22/secrets/sdm.env` は初期入力用保管庫として位置づけ済み。  
- go2rtc仕様: v1.9.9+ 前提、`nest://{sdm_device_id}?project_id=...&enterprise=...&client_id=...&client_secret=...&refresh_token=...` を `/api/streams` へ登録する方針を明記。秘密値はDBに埋め込まない。  
- RBAC/監査/CSRF: `/api/sdm/*` 管理者限定、CSRF必須（Cookie運用時）、監査ログに秘密値をマスクして記録する方針を明記。  
- 登録フロー整合性: `enabled=false` でINSERT→go2rtc登録成功後に `enabled=true` へ更新、失敗時はロールバック。  
- トークンライフサイクル: 状態モデル（not_configured/configured_pending_auth/authorized/error）、401/403でerror遷移、429は待機、指数バックオフを明記。  
- データ制約: camera_id文字種・長さ、`sdm_device_id` ユニークインデックス、`sdm_traits` 上限を定義。  
- 解除/削除フロー: `DELETE /api/sdm/devices/:id` で無効化＋go2rtc削除＋reconcile再試行を設計。  
- Snapshot方針: go2rtc frame.jpeg を既定（1280x720）で取得、必要時にwidth/height指定、Polling間隔は既存設定を流用し帯域はメトリクス監視。  
- テスト計画: 正常/異常/退行（refresh_token失効、go2rtc停止、空リスト、再認可、E2Eウィザード）まで記載。

## 残リスク/フォローアップ
- go2rtc nest ソース文字列は v1.9.9+ を前提とした設計値。実環境のgo2rtcビルドが対応しているかを実機で検証し、必要なら互換文字列・バージョン固定を追加で明記する。  
- `DELETE /api/sdm/devices/:id` は設計のみで実装未。実装時に監査ログと idempotency を確認する。  
- Camscanとの導線はCamscan修正完了後にIssue化して実装する（プレースホルダ禁止）。
- ウィザード内のフールプルーフ文言を実装に反映する際、スクリーンショットやYes/NoチェックリストのUXを設計どおりに落とす必要がある。

## MECE/アンアンビギュアス確認
- 領域: データ/設定、API・処理、運用・セキュリティ、テストの4カテゴリで重複なく整理済み。残リスクは上記に隔離し、本編に反映済みの内容は矛盾なし。  
