# 整理チェックリスト
- [x] Move docs/common/直近の方針.md -> docs/common/outdated/直近の方針.md
- [x] Move docs/common/external_systems.md -> docs/common/outdated/external_systems.md
- [x] Move docs/common/dataflow_specs.md -> docs/common/outdated/dataflow_specs.md
- [x] Move docs/common/default_SSIDPASS.md -> docs/common/outdated/default_SSIDPASS.md
- [x] Move docs/common/important_designersMemo.md -> docs/common/outdated/important_designersMemo.md
- [x] Move docs/is10_stability_investigation_report.md -> docs/common/outdated/is10_stability_investigation_report.md
- [x] Move docs/mobes2.0/mobes_devprompt.md -> docs/mobes2.0/outdated/mobes_devprompt.md
- [x] Move docs/mobes2.0/mobes側直近指示.md -> docs/mobes2.0/outdated/mobes側直近指示.md
- [x] Update docs/common/outdated/important_designersMemo.md with latest risk/TODO notes
- [x] Update docs/common/outdated/直近の方針.md (Before/After + TODO)
- [x] Update docs/common/outdated/external_systems.md (Before/After + TODO)
- [x] Update docs/common/outdated/dataflow_specs.md (Before/After + TODO)
- [x] Update docs/common/outdated/default_SSIDPASS.md (Before/After + TODO)
- [x] Update docs/common/outdated/is10_stability_investigation_report.md (Before/After + TODO)
- [x] Update docs/mobes2.0/outdated/mobes_devprompt.md (Before/After + TODO)
- [x] Update docs/mobes2.0/outdated/mobes側直近指示.md (Before/After + TODO)

# 修正文書チェックリスト（本文を現行実装に整合させる）
- [x] docs/common/outdated/直近の方針.md
- [x] docs/common/outdated/external_systems.md
- [x] docs/common/outdated/dataflow_specs.md
- [x] docs/common/outdated/default_SSIDPASS.md
- [x] docs/common/outdated/is10_stability_investigation_report.md
- [x] docs/common/outdated/important_designersMemo.md
- [x] docs/mobes2.0/outdated/mobes_devprompt.md
- [x] docs/mobes2.0/outdated/mobes側直近指示.md

# docs 構造と整理計画

## 変更計画（ディレクトリ/ドキュメント再整理）
- common/ に「outdated/」サブディレクトリを作り、要修正フラグの文書を一時退避（直近の方針、external_systems、dataflow_specs、default_SSIDPASS、important_designersMemoの未反映箇所、mobes2.0系古い指示、is10_stability_investigation_report）。
- 退避前に各「要修正」ドキュメントへ最新版で不足している項目のTODOを先頭に追記（WDT/バックオフ/断片化対策、最新APIパス、リンク検証など）。
- 現行整合ドキュメントは現位置維持。案件固有（is10_0150…）は devices/ へ集約し、案件別フォルダを検討。
- common/default_SSIDPASS.md は空なので、運用決定後に値を記入するか、明示的に「未設定」と記す。
- designdetails/ISMS仕様書図面0718.pdf は内容確認後、最新版かどうかを記載。古ければ outdated/ へコピーのうえリンクを更新。
- is10_stability_investigation_report は日付とバージョンを明記し、最新版再測予定をTODOとして追記。

# docs 構造
- common/ … 共通仕様・事故・外部連携・認証・UIガイド
- ESP32/global/ … 共通モジュール仕様・ビルド/設定スキーマ
- is03/ … Zero3受信サーバ関連
- is04a/・is05a/ … 各デバイスのテスト待ち/レビュー依頼
- is10/・devices/ … ルーター監視(is10)系仕様・案件別設定
- mobes2.0/ … mobes側メモ
- img/・designdetails/ … ロゴ・図面
- ルート直下 … 案件固有設定等

---
各ドキュメントの要約（約400文字目安）と整合性メモ。不整合・要補正は「要修正」等を明示。

## docs/is05a/PENDING_TEST.md（2024-12-22, 現行・未テスト）
is05aテスト待ち。最新修正（<script>追加、APIパス統一、認証追加、ArduinoJson化、経過時間デバウンス）を列挙。Gate準備後の基本/API/StateReporter/Webhookテストをチェックリスト化。Open Point: `/api/webhook/test`がplatform無視、sendHeartbeatが1秒間隔制限共有。実装最新と整合、未テスト注意。

## docs/devices/is10_openwrt_router_inspector.md（現行整合）
is10概要・設定リファレンス。Type/Product/FID/TID/テナント情報、OpenWrt/ASUSWRTをSSH巡回してWAN/LAN/SSID/クライアント取得。globals/routers設定JSON例、ピン配置、動作モード、取得コマンド例、deviceStateReport構造とsanitizeを記載。現行実装と矛盾なし。

## docs/is10/api_guide.md（現行整合）
is10 HTTP API完全版。共通APIと`/api/is10/*`（polling/router/import/inspector等）＋OTA APIを例示。バリデーション(tid/fid/ipAddr等)、エラー、運用フロー、デフォルト設定を網羅。大容量JSONの注意あり。HttpManagerIs10と整合。

## docs/is10/______MUST_READ_IS10_CONFIGURATION______.md（現行整合）
is10量産・展開の必読。min_spiffs必須/huge_app禁止、AraneaSettings編集箇所（TID/FID/テナント認証/WiFi）明示。MCPのコンパイル/アップロード/OTA例、WebUI/MQTTでのルーター設定、ASUSWRT向け長タイムアウト推奨。最新実装と合致。

## docs/is10_0150HALE_kyoto_tanba_config.md（現行・案件固有）
HALE京都丹波口案件。ASUS RT-AC-59U 16台のRID/WAN/LAN/SSH(65305)表と共通SSH認証を記載。ASUSWRTはPing不可/遅い→タイムアウト長め推奨。現場リファレンスとして有効。

## docs/is03/dev_prompt.md（現行・機密）
is03構築プロンプト。固定値（Armbianイメージ/TID/FID/IP等）と必須入力(TENANT_PRIMARY_LACISID)を明示。DHCP→固定IP設定、依存パッケージ、is03-relay仕様（BLEスキャン→リング2000→SQLiteバッチ10s or 50件、DB上限20000件、WS/HTTP API）、systemd自動起動、Gate登録curl手順を詳細化。現行と整合、機密注意。

## docs/is03/overview.md（現行整合）
is03の役割と手順を平易に整理。is01送信/is03受信/is02中継の関係、SD書き込み→起動→ユーザー/パッケージ→固定IP→is03-relay→systemd→テスト→ブラウザ確認の流れ。リング2000件＋バッチ書込みでSD保護を強調。現行方針と一致。

## docs/is04a/PENDING_TEST.md（現行・未テスト）
is04aテスト待ち。Gate準備後の基本/API/StateReporter/HTTPパルス等チェックリストとOpen Pointsを記載。入力極性修正・固有API分離・パルス開始送信など最新実装に整合。未テスト注意。

## docs/common/outdated/is10_stability_investigation_report.md（要修正・移動済み）
is10安定性調査レポート（日付不明）。最新ファームとの差異が高く、再現テストと更新が必要。日付・バージョンを追記し、最新検証結果で改訂すること。

## docs/common/araneaDeviceGate_spec.md（現行整合）
Gate挙動仕様。HTTPコード別返却（既存/停止/削除/recovered/新規）、同一MACでProductType変更時の旧デバイス削除→再登録、所有権変更時のCIC再発行と監査ログ仕様を詳細化。最新Gate前提。

## docs/common/tenant_credentials.md（現行・機密）
市山水産向けTID/FID/Primary認証情報を表形式で記載し、Gate登録/stateReportのJSON例を提示。値は2025系で現行。機密のためアクセス制御必須。

## docs/common/outdated/important_designersMemo.md（要修正・移動済み）
ESP32設計指針。SettingManager集約、ハードコード禁止、各デバイス役割/LacisID形式/relay優先順を整理。現行コードに未反映の事項あり：最新のWDT/バックオフ/メモリ断片化対策、is04a/is05aの固有エンドポイント分離やforce送信運用が未記載。要更新。

## docs/common/pin_specs.md（現行整合）
ESP32 DevKitCの入力専用/ストラップ/禁止ピン、I2C標準。is01/02/04/05のGPIO割当（ブートストラップ注意）とチャタリング例、pin_checkツールの使い方を記載。最新実装と整合。

## docs/common/aranea_web_ui_implementation_guide.md（現行整合）
AraneaWebUI拡張ガイド。基底構造体/共通タブ/共通APIとBasic/CIC認証の扱い、typeSpecificステータス・タブ・JS追加、独自API登録手順を詳述。派生HttpManager実装の標準資料。現行と一致。

## docs/common/outdated/直近の方針.md（要修正・移動済み）
離島運用前提の方針メモ。productCodeは4桁推奨（ISMS末尾は避ける）、現地優先構成や共通モジュール案を記述。日付が古く、最新計画・実装（is04a/is05a修正、バックオフ/WDT方針等）に合わせた更新が必要。

## docs/common/incident_is10_ssh_crash.md（現行・事故記録）
is10のSSHクラッシュインシデント報告。原因と再発防止策（タイムアウト設定、セッション管理）を整理。最新実装でも参考にすべき事故情報。

## docs/common/outdated/external_systems.md（要修正・移動済み）
外部システム概要。mobes2.0とArduino-MCPを紹介。リンクの有効性未確認。最新の運用手順・URLに合わせて更新が必要。

## docs/common/device_provisioning_flow.md（現行整合）
プロビジョニングフロー。大量生産（AraneaSettings埋め込みで自動登録）と汎用（AP UI）の二形態を説明。設定例とメリットを記載。現行手順と合致。

## docs/common/is10_implementation_record.md（2025-12-20, 現行整合）
is10実装記録。16台SSH完走ログ（約38s、ヒープ-3.5KB）。禁止事項: huge_app禁止、WDT/セマフォ介入禁止、余計な監視追加禁止、SSHタスクはCore1/stack51KB/優先度+1、loopブロッキング禁止。現行安定構成の基準。

## docs/common/aranea_web_ui_design.md（現行整合）
AraneaWebUIデザイン指針。タブ構成、CSS/JSルール、スタイル統一方針を記述。現行UIと一致。

## docs/common/ota_procedure.md（現行整合）
OTA手順標準化。パーティションスキーム、HTTP OTA操作、失敗時注意を整理。最新コードを前提に有効。

## docs/common/system_integration.md（現行整合）
is01〜is05/is03の連携を明文化。lacisId形式(3+Type+MAC12+0096)、productType割当(001〜007)。is01は初回のみWiFiでCIC取得・通常BLEのみ、is02/05/04はZero3中継優先、is03は受信/2000件リング/バッチ書込みで可視化。クラウド未完成時の運用前提を示す。

## docs/common/outdated/dataflow_specs.md（要修正・移動済み）
センサー→ゲートウェイ→Zero3/クラウドの高レベルデータフロー。更新時期不明。最新実装（is04a/is05aの固有送信先やバックオフ方針）に合わせて補足が必要。

## docs/common/outdated/default_SSIDPASS.md（要修正・空・移動済み）
デフォルトWiFi SSID/パスが未記載。運用で必要な値を追加する必要あり。

## docs/ESP32/global/00_overview.md（現行整合）
共通モジュール概要と利用ポリシーを説明。最新コードに対応。

## docs/ESP32/global/01_directory_layout.md（現行整合）
共通ライブラリと各スケッチのディレクトリ構造を説明。ビルド/参照時のガイド。

## docs/ESP32/global/02_build_flash_arduino_cli.md（現行整合）
Arduino CLI/MCPでのビルド/書き込み手順、パーティション設定、コマンド例を記載。現行フローに合致。

## docs/ESP32/global/03_settings_schema.md（現行整合）
SettingManagerのキー/デフォルトスキーマ、NVS/SPIFFSの型・用途を整理。現行実装に準拠。

## docs/ESP32/global/04_ble_payload_spec.md（現行整合）
BLEペイロード仕様（is01→is02/03）。LacisID組み立て、フィールド構成、拡張余地を説明。現行設計の基準。

## docs/ESP32/global/05_is01_spec.md（現行整合）
is01仕様書。動作モード、電力設計、送信ペイロード、DeepSleep運用を整理。現行コードと整合。

## docs/ESP32/global/06_is02_spec.md（現行整合）
is02ゲートウェイ仕様。受信→送信フロー、クラウド/Zero3連携、設定項目を記載。最新実装と概ね一致。

## docs/ESP32/global/07_test_plan.md（現行整合）
ESP32共通テスト計画。機能/統合/耐久項目を列挙。is04a/is05aにも転用可能。

## docs/mobes2.0/outdated/mobes_devprompt.md（要修正・移動済み）
mobes2.0開発者向けプロンプト/手順メモ。時期依存で現状運用と差異がある可能性。最新タスク/手順に合わせて更新が必要。

## docs/mobes2.0/outdated/mobes側直近指示.md（要修正・移動済み）
mobes側の直近タスク・指示メモ。更新時期が古く現状と乖離の恐れ。最新状況に合わせた改訂が必要。

## docs/img/aranea_logo.svg / aranea_logo_icononly.svg
ロゴ。現行利用可。

## docs/designdetails/ISMS仕様書図面0718.pdf
図面PDF。内容未確認につき最新版か要確認。

## docs/Untitled
ロゴファイル名のメモのみ。重複情報で整理候補だが害なし。
