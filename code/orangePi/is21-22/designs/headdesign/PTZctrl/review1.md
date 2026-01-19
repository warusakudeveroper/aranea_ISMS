• PTZctrl_BasicInstructionsに対する再確認結果です（MECE、アンアンビギュアスに記載）。

  - PTZ操作が実機に届く実装が無い：code/orangePi/is22/src/ptz_controller/service.rs:174-236 がONVIF呼び出しをスタブ
    化しており、Move/Stop/Homeはログを出すだけでカメラは動作しません。API側でもcode/orangePi/is22/src/web_api/
    ptz_routes.rs:20-24,52でLease認証がコメントアウトのままです。クリックモーダルでのPTZ操作要件を満たせていません。
  - クリックモーダル拡大（約130%）未対応：code/orangePi/is22/frontend/src/components/LiveViewModal.tsx:148-210 は
    max-w-5xlの通常サイズ＋回転のみで、拡大スタイルが追加されていません。
  - PTZ操作フィードバック不足：code/orangePi/is22/frontend/src/components/ptz/PtzOverlay.tsx:73-153 は矢印の強調のみ
    で「← pan left」等の一時表示やカーソル近傍の明示的フィードバックがありません。
  - 「PTZctrlを無効にする」のグレーアウト未対応：code/orangePi/is22/frontend/src/components/
    CameraDetailModal.tsx:746-792 はPTZ対応カメラでのみセクションを表示し、非対応カメラではセクションごと非表示のま
    まです。要求どおり「非対応ならグレーアウトしたチェック欄」が提供されていません。
  - サジェストビューの重複防止・上位移動・最大3件制限が未達：
      - 既存カメラ判定がisExitingを除外しているため（SuggestPane.tsx:95-187）、onairTime切れでisExitingになった直後
        に同じカメラの新イベントが来ると新規エントリが追加され、同一カメラが複数表示されます。
      - 表示リストvisibleCamerasがonAirCameras全件（isExiting含む）をそのまま描画しており（SuggestPane.tsx:266-
        338）、終了待ちを含めて4枚以上表示され得ます。
      - 既存エントリをトップに浮かせる処理はisExiting対象を拾わないため、長時間運用時の再検知で上位移動／重複禁止仕
        様を保証できていません。
  - LiveView優先・AccessAbsorber連携なし：LiveViewもサジェストもGo2rtcPlayerで直接ストリーム取得するだけで
    （LiveViewModal.tsx:190-209、SuggestPane.tsx:320-338）、重複ストリーム禁止・優先切替・ユーザーフィードバック表示
    がありません。クリックモーダル中にサジェストから同一カメラが来ても遮断・メッセージ表示が行われません。

  実施済みと確認できた項目

  - 回転設定の反映はタイル/サジェスト/LiveViewでCSS回転を適用済み（例：CameraTile.tsx:217-241、SuggestPane.tsx:320-
    333、LiveViewModal.tsx:190-210）。
  - onAirTimeに基づく定期チェックと削除キューは存在（SuggestPane.tsx:225-255）。

  テスト実行：未実施（コードリーディングのみ）。

  対処の目安

  1. PTZバックエンドに実体のONVIF/Tapo制御を実装し、ルートでLease認証を有効化。
  2. LiveViewのUIスケールを設計どおり約130%へ拡大。
  3. PTZオーバーレイに方向フィードバックと回転追随を追加。
  4. 非PTZカメラでPTZ無効チェックをグレーアウト表示（DB値も保存可能に）。
  5. SuggestPaneの重複・上限・再検知ロジックをisExitingを含めて正規化し、常時最大3件・同一カメラ1件・再検知は上位移
     動に統一。
  6. LiveView優先の重複ストリーム禁止をAccessAbsorber等で実装し、サジェスト側に明示的なフィードバックを出す。


  # 追加レビュー1

  • 以下、PTZctrl_BasicInstructionsの全項目を対象に再確認しました（ドキュメント/実装/依存を網羅）。現時点で未解決のポ
  イントを優先的に列挙します。

  - PTZ操作が実機に届かない：code/orangePi/is22/src/ptz_controller/service.rs:174-236 でMove/Stop/HomeはONVIF呼び出
    しがスタブのまま（ログのみ）。code/orangePi/is22/src/web_api/ptz_routes.rs:20-24,52 でもLease認証がコメントアウ
    トされており、モーダル権限連携も未実装。クリックモーダルでのPTZ操作要件を満たせていません。
  - クリックモーダルの拡大（約130%）未対応：code/orangePi/is22/frontend/src/components/LiveViewModal.tsx:148-210 は
    等倍表示のまま（max-w-5xl、transformは回転のみ）。BasicInstructionsの「現在の130%を目処にUI表示を大きくする」が
    未反映。
  - PTZ操作フィードバック不足：code/orangePi/is22/frontend/src/components/ptz/PtzOverlay.tsx:73-153 は矢印のハイライ
    トのみで、「← pan left」等の即時表示やカーソル近傍の明示的フィードバックがありません。追加仕様2未達。
  - 「PTZctrlを無効にする」チェックのグレーアウト未対応：code/orangePi/is22/frontend/src/components/
    CameraDetailModal.tsx:746-792 はPTZ対応カメラのみ表示し、非対応カメラではセクションごと非表示です。追加仕様1の
    「PTZ機能のないカメラではグレーアウト」に未対応。
  - サジェストビューの重複防止・上位移動・最大3件制限が未達：
      - 既存判定がisExitingを除外するため、終了アニメーション中に同一カメラ再検知で新規追加され、同一カメラが複数並
        び得ます（code/orangePi/is22/frontend/src/components/SuggestPane.tsx:95-187）。
      - 表示はonAirCameras全件（isExiting含む）を描画するため、終了待ちを含め4枚以上表示され得ます
        （SuggestPane.tsx:266-338）。最大3件仕様を保証できていません。
      - 再検知時の上位移動もisExitingを除外しており、長時間運用で仕様どおりの挙動が担保されません。
  - LiveView優先／AccessAbsorber連携なし：LiveView/SuggestともにGo2rtcPlayerで直接取得するだけで、重複ストリーム禁止
    や優先切替・ユーザーフィードバックがありません（例：LiveViewModal.tsx:190-210、SuggestPane.tsx:320-338）。追加仕
    様「クリックモーダル中はサジェスト側を閉じる/メッセージ表示」が未実装。
  - PTZ回転追随はサーバ側のapply_rotationで処理されていますが、PTZ UI自体の視覚方向や操作フィードバックには回転追随
    がなく、追加仕様3の「コントローラ追随」はUI面で未達成です。

  実装済みと確認できた点

  - 回転設定の表示反映：タイル/サジェスト/LiveViewでCSS回転を適用（例：CameraTile.tsx:217-241、SuggestPane.tsx:320-
    333、LiveViewModal.tsx:190-210）。
  - onAirTimeに基づく定期チェックと削除キューは存在（SuggestPane.tsx:225-255）。

  総括
  上記未達が解消されていないため、PTZctrl_BasicInstructionsの要件は未完了です。特に「実機PTZ制御の実装」「モーダル拡
  大」「サジェスト重複/上限/優先制御」「非対応時のグレーアウト」「操作フィードバック」の着手が必要です。

  # 追加レビュー2

  • PTZctrl_BasicInstructionsの達成状況（詳細検証）

  1. PTZ実機制御・Lease認証

  - バックエンド: code/orangePi/is22/src/ptz_controller/service.rs:174-236 はONVIF実行部がスタブ（コメントのみ、実機
    制御なし）。execute_ptz_stop_static/execute_ptz_homeも同様。
  - ルート: code/orangePi/is22/src/web_api/ptz_routes.rs:20-24,52,78-82 でLease認証がコメントアウトのまま。
    → クリックモーダルからのPTZ操作はサーバーで実際の制御を行わず、要件「PTZ対応カメラのクリックモーダルで操作可能」
    を満たしていません。

  2. クリックモーダル拡大（約130%）

  - code/orangePi/is22/frontend/src/components/LiveViewModal.tsx:148-244 は従来のmax-w-5xl・aspect-videoのまま。拡大
    係数指定やズーム無し。
    → 「130%を目処にUI表示を大きくする」が未実装。

  3. 画像/映像の回転適用

  - タイル: code/orangePi/is22/frontend/src/components/CameraTile.tsx:217-241 でCSS rotate適用。
  - サジェスト: code/orangePi/is22/frontend/src/components/SuggestPane.tsx:320-333 でRTSP表示に回転適用。
  - LiveView: code/orangePi/is22/frontend/src/components/LiveViewModal.tsx:190-211 で映像に回転適用。
    → 既知問題②は解消済み。

  4. サジェストビュー重複禁止・上位移動・最大3枚・onAirTime順守

  - 既存カメラ検出: SuggestPane.tsx:95-187 はisExitingを除外して一致判定。終了アニメーション中に同一カメラ再検知で新
    規追加され、重複発生。
  - 上限処理: 新規追加後にactiveList（isExiting除外）で4件目以降を除外するが、描画はvisibleCameras = onAirCameras全
    件（266-338）なので、isExitingが残存すると4件以上表示。
  - onAirTime: 定期チェックは実装済み（225-255）だが、上記重複ロジックにより長時間運用で仕様逸脱が継続。
    → 既知問題③④⑤は未解消。

  5. LiveView優先・重複ストリーム禁止・フィードバック

  - LiveView: LiveViewModal.tsx:190-244 はgo2rtc再生のみ。
  - サジェスト: SuggestPane.tsx:320-338 もgo2rtc再生のみ。
  - AccessAbsorber/重複抑止/フィードバック（「重複ストリーム禁止で閉じる」メッセージ）は未実装。
    → 既知問題⑥未解消。

  6. 「PTZctrlを無効にする」チェックの追加仕様

  - UI: CameraDetailModal.tsx:746-792 はPTZ対応カメラのみ表示し、非対応カメラではセクション非表示のまま。グレーアウ
    トチェック欄なし。
  - DB/型: ptz_disabled項目は残存するがUI挙動が追加仕様に未対応。
    → 追加仕様①未達。

  7. PTZ操作フィードバック

  - PtzOverlay.tsx:43-199 で方向ラベル（例: “← pan left”）を1.5秒表示する仕組みを追加。
    → 追加仕様②（明確なフィードバック）は反映済み。

  8. PTZ UI回転追随

  - PtzOverlay.tsx:21-118 でrotation受け取り、パネルを回転。
  - LiveViewModal.tsx:232-243 でrotation={camera.rotation||0}を渡す。
  - サーバー側も方向を回転補正（ptz_controller/service.rs:57-65）。
    → 追加仕様③のUI追随は実装済み。

  総合判断

  - 未完了: ①実機PTZ制御/Lease認証、②モーダル拡大(130%)、④サジェスト重複・上限・上位移動、⑤LiveView優先の重複スト
    リーム抑止とフィードバック、⑥非PTZカメラでのグレーアウトUI。
  - 完了確認: 画像回転反映、PTZ UI回転追随、操作フィードバック表示。

  PTZctrl_BasicInstructionsの全項目はまだ満たされていません。

  # 追加レビュー3
  • PTZctrl_BasicInstructions対応の再レビュー結果です。新たに加わったAccessAbsorberプリエンプションやPTZ UI回転追随を
  確認しましたが、以下の未達・リスクが残っています。

  未達/要修正

  - 実機PTZ制御が依然スタブ：code/orangePi/is22/src/ptz_controller/service.rs:174-236 のMove/Stop/HomeはONVIF呼び出
    しを実行せずログのみ。code/orangePi/is22/src/web_api/ptz_routes.rs:20-24,52,78-82 もLease認証がコメントアウトの
    まま。クリックモーダルから実カメラは動きません。
  - クリックモーダル拡大(約130%)未実装：code/orangePi/is22/frontend/src/components/LiveViewModal.tsx:148-244 は従来
    サイズのまま。拡大表示が行われていません。
  - PTZ無効チェックのグレーアウト未対応：code/orangePi/is22/frontend/src/components/CameraDetailModal.tsx:746-792 で
    はPTZ非対応カメラにチェック欄が表示されず（セクションごと非表示）、追加仕様「非対応ならグレーアウト」が未達。
  - サジェストビューの「最大3枚・重複禁止・再検知で上位移動」未保証
      - 判定はisExitingを含めて同一カメラを1件に寄せるよう改善（復活処理あり）が、表示リストはonAirCameras全件を描画
        するため、isExitingが残ると4枚以上表示され得ます（code/orangePi/is22/frontend/src/components/
        SuggestPane.tsx:266-338）。
      - 上限判定はisExitingを除外したactiveのみで行うため、可視枚数を3に強制できていません。
  - LiveView優先・重複ストリーム禁止が未完
      - AccessAbsorberプリエンプションはSuggest側に導入されましたが、LiveView（クリックモーダル）からAccessAbsorber
        を取得するコードがありません（フロントに/api/access-absorber/streams/acquire呼び出し無し、ClickModal用途の導
        線不在）。
      - そのため、クリックモーダルとSuggestが同一カメラで同時再生した場合に必ずプリエンプトする保証がありません。
      - WebSocketのstream_preemptedを受けたSuggestPane側のオーバーレイ表示・カウントダウン終了処理は実装済み
        （SuggestPane.tsx:203-340,620-780）ですが、発火する前提のクリックモーダル側連携が欠落しています。
  - PTZ操作結果フィードバックは表示されるが、実機制御が無いので結果と整合しません（操作成功/失敗の真偽が取れない）。

  改善済み

  - PTZ UI回転追随：code/orangePi/is22/frontend/src/components/ptz/PtzOverlay.tsx:21-119 でパネル回転、
    LiveViewModal.tsx:232-243 からrotation渡し。映像と矢印の向きが一致。
  - PTZ操作フィードバック表示：PtzOverlay.tsx:6-200 で方向ラベルを1.5秒表示。
  - サジェストAccessAbsorber統合＆プリエンプション通知：code/orangePi/is22/frontend/src/components/
    SuggestPane.tsx:128-780、hooks/useWebSocket.ts:24-186 でWS通知処理、バックエンドのプリエンプション情報・ブロード
    キャスト実装（access_absorber/types.rs:510-545、service.rs:473-535、web_api/access_absorber_routes.rs:18-133、
    realtime_hub/mod.rs:45-279）。

  結論
  PTZctrl_BasicInstructionsの核心（実機PTZ制御、モーダル拡大、サジェスト最大3枚・重複禁止、LiveView優先の重複抑止、
  非対応時グレーアウト）は未完了です。回転追随と操作フィードバック、AccessAbsorberプリエンプション通知は実装されてい
  ますが、要件全体の達成には上記未達の解消が必要です。