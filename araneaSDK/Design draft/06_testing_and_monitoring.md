# 06 テストと監視（開発効率向上）

## 認証テスト
- `cli/auth test --env dev --lacisId ... --userId ... --cic ...`  
  register/report 両方の認証成否を即時確認。再試行/バックオフもログ化。

## HTTP エンドポイントテスト
- `cli/register test` でダミー登録→レスポンスと保存値（NVS/ファイル）を検証。
- `cli/report test --payload fixtures/... --endpoint http` で deviceStateReport を送信し、応答コードとサーバ反映を確認。

## MQTT テスト
- `cli/mqtt echo --env dev --topic aranea/{tid}/{lacisId}/state --payload ...`  
  pub/sub 往復、WS/TLS、LWT、再接続、throughput を測定。

## mobes 側監視（Firestore / BigQuery）
- `cli/report verify-firestore --env dev --lacisId ...`  
  userObject/detail/report への反映を確認し、欠落/遅延を検知。
- `cli/report verify-bq --table ...`  
  直近インサートを確認し、遅延/欠落を検出。
- CI ワークフロー: dev で register→report→MQTT→Firestore/BQ 検証まで一連実行し、結果を両リポジトリで共有。

## 観点
- ESP32 / Linux / Web の多系統で同じ payload を送信し、結果ログをハッシュ付きで比較（同一プロトコルを保証）。
- バックオフ/最小送信間隔などの挙動差を検出し、実装差分を早期に洗い出す。
