# Paraclate 要件定義
作成日: 2026-01-XX  
対象: is21（ParaclateEdge）、is22（Paraclate Server）、mobes2.0 Paraclate APP  
準拠: Paraclate_DesignOverview.md / investigation_report 一式 / The_golden_rules.md

## 1. 目的と背景
- 開発名MobesAIcamControlTowerを **Paraclate** に改名し、監視・監査ではなく「穏やかに見守り、対話的に報告する」カメラシステムを実現する。
- is22をParaclate Server、is21をParaclateEdgeとして位置付け、mobes2.0のParaclate APPと連携する。
- lacisOath権限境界（tid/fid）とSSoT原則を維持したまま、Summary/GrandSummaryとAIアシスタント機能を統合する。

## 2. スコープ
- 対象: is22バックエンド/フロント、is21推論拡張、mobes2.0 Paraclate APP連携、登録・認証・サマリー送信・チャット回答のためのI/F。
- 非対象: LLMモデル選定/チューニング自体、既存カメラRTSP制御の仕様変更、外部予約/部屋/シフトサービスの新規開発。

## 3. 役割と前提
- **is22 Paraclate Server**: カメラ台帳SSoT、検知ログSSoT、Summary生成/送信、設定同期、BQ同期、AIチャット用データ提供。
- **is21 ParaclateEdge**: 推論エンジン（YOLO+PAR+顔特徴+車両OCR拡張）、is22からの起動/設定要求に応答。
- **Paraclate APP (mobes2.0)**: テナント管理層UI、Paraclateエンドポイント・スケジュール設定SSoT、Summary閲覧・チャットUX、越境権限(blessing)管理。
- **ID/認証**: lacisID形式`[Prefix=3][ProductType=3桁][MAC12][ProductCode4]`を踏襲。TypeDomain=araneaDeviceを正とし、is22=022/0000、カメラ=is801（ProductCodeはブランド割当）。認証はlacisOath（lacisId/userId/cic、pass不要）。blessingはpermission91+のみ発行し越境時のみ使用。
- **テナント前提**: 運用tidをmijeo.inc `T2025120621041161827`（lacisID:18217487937895888001、CIC:204965）へ移行。
- **SSoT宣言**: テナント境界/権限=mobes2.0、カメラ台帳/検知ログ/サマリー=is22、長期分析=BigQuery、AI設定/Paraclate endpoint=mobes2.0 AI Model Settings、画像保存=LacisFiles。

## 4. 機能要件（FR）
- **FR-01 名称統一**: is21/22ドキュメント・UIをParaclate/ParaclateEdge表記とし、サブタイトルを「mobes AI control Tower」とする。
- **FR-02 デバイス登録（AraneaRegister）**: is22が起動時にaraneaDeviceGateへ登録しCIC/stateEndpoint/mqttEndpointを取得・保持する。再起動時は既存CICを再利用。登録APIはPOST/GET/DELETEを提供。
- **FR-03 カメラ登録**: IpcamScan承認時にカメラをaraneaDeviceとして登録し、`3801{MAC}{ProductCode(ブランド)}`のlacisIDを発行・DB(cameras)へ保存。lacisID未付与状態でのpreset同期は禁止。
- **FR-04 設定管理SSoT**: Paraclate endpoint `https://www.example_endpoint.com`（暫定）、Summary間隔、GrandSummary時刻、保持期限などをmobes2.0 AI Model Settings（aiApplications.paraclate）で管理し、is22側はキャッシュ・反映のみ。設定の双方向同期APIとPub/Sub通知を持つ。
- **FR-05 Summary生成**: is22が検知ログからSummary（間隔ベース=hourly）を生成しai_summary_cacheへ保存。camera_contextはcamerasテーブルのSSoTを参照し、summary_json等で送信用ペイロードを保持。
- **FR-06 GrandSummary生成**: 設定された時刻ベース（daily）で複数Summaryを統合しai_summary_cacheへ保存。Emergencyタイプもseverity閾値で即時生成。
- **FR-07 Summary送信**: Summary/GrandSummaryをParaclate APPへ送信するエンドポイントをis22に実装し、lacisOathで認証。blessingは越境時のみ付与。送信フォーマットはDesignOverview記載のJSONを踏襲。
- **FR-08 AIチャット用データ提供**: is22はAIチャットのための事実取得API（detection_logs、cameras、summaries、ai_chat_history保存）を提供し、LLM推論自体はParaclate APP側で実行する。
- **FR-09 is21連携**: is22がis21をtid/lacisID/cicでアクティベートし、推論結果（車両OCR・顔特徴を含む）を受信・保存。is21はシステムテナントT999... fid=0000 permission71のまま応答。
- **FR-10 BQ同期**: is22のbq_sync_queueを用い、検知ログおよびSummaryをBigQueryへバッチ同期しsynced_to_bqを管理する。
- **FR-11 画像保存**: 検知スナップショットはLacisFilesに保存しTTL/監査ログ運用を遵守。is22からの直書き禁止、署名URLまたはサーバ側アップロードで行う。
- **FR-12 モニタリング/状態**: Paraclate連携状態、最後のSummary時刻、カメラ数、接続状態を取得するステータスAPIをis22に設ける。Frontend AIタブで表示。
- **FR-13 UX要件（AIアシスタント）**: 対話口調は「穏やかに見守る」スタイル。不確実性は明示し、時間範囲を会話で伸縮できる。回答には参照IDを含める。

## 5. 非機能要件（NFR）
- **NFR-01 セキュリティ/権限**: tid/fid境界を強制。blessing利用時は発行者(permission91+)・対象lacisID・有効期限を必須とし監査ログを残す。越境はサーバ側のみで処理。
- **NFR-02 可用性/運用**: Summaryスケジューラは失敗時リトライと次回実行更新を持つ。BQ同期はリトライとDLQ相当のfailed管理を行う。
- **NFR-03 拡張性/SOLID**: 登録/サマリー/チャット/同期/設定モジュールは責務分離（単一責任）、インターフェース定義で開放閉鎖・置換を担保（例: AI実行はParaclate APPへ委譲）。
- **NFR-04 可観測性**: Summary生成・送信、登録、BQ同期、Pub/Sub同期でイベントログ・メトリクスを残し、障害解析を可能にする。
- **NFR-05 性能**: Summary生成は標準環境で60分ウィンドウを1分以内に処理可能とする目標。チャット用検索APIは1秒以内の応答を目標（キャッシュ/索引設計前提）。
- **NFR-06 一貫性**: SSoTを厳守し二重保存を避ける。設定はmobes2.0が正とし、is22はキャッシュに留める。

## 6. 依存・制約
- araneaDeviceGateがType/permission定義を持つこと（デフォルト権限未設定時の登録失敗に留意）。
- mobes2.0側でParaclate APPおよびAI Model Settings拡張が必要（endpoint等のフィールド追加）。
- BQ/Firestore/LacisFilesのプロジェクト権限設定が必要。
- is22現行実装ではAraneaRegister・Summary API・Paraclate連携が未実装であるため、本要件を実装順に落とし込むこと。

## 7. 受入/テスト方針（高位）
- is22登録→CIC取得→再起動で再利用できること。
- カメラ承認時に20桁lacisIDが必ず発行され、preset同期が通ること。
- Summary/GrandSummaryが設定に従い生成・ai_summary_cache保存・Paraclate APPへ到達すること。
- Paraclateチャットで「赤い服の人来なかった？」に対し、該当イベントまたは不明明示を返し、根拠IDを提示すること。
- blessing無しで越境アクセス不可、blessing有効時のみ許可され監査が残ること。
- 画像がLacisFilesに保存されTTL削除が機能すること。

## 8. MECE/Solid確認
- Auth/Registry、設定、生成、送信、データ提供、同期、UX、セキュリティの各領域を重複なく分割しMECEに整理。SSoTを明示し二重管理を禁止。
- モジュール責務を分離し、登録クライアント・サマリー生成・送信クライアント・データ提供API・同期サービスが相互依存を限定する形で設計（SOLIDのSRP/DIP準拠）。
