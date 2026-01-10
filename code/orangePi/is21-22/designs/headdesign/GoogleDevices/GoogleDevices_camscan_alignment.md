# GoogleDevices × Camscan 統合設計メモ（SDM/Nest / IpcamScan整合）

目的: Google/Nest(SDM)統合設計(GD-01〜03)と Camscan(IpcamScan)設計群の**整合と統合ポイント**を明文化する。IpcamScan修正作業と競合しないよう、SSoT・責務境界・UI導線を決定する。

## 0. SSoTと責務境界（変更なし／明文化）
- カメラ台帳: `cameras` テーブル（`family`/`discovery_method`/`sdm_device_id`等）。Nestは `family=nest`, `discovery_method=sdm`, `snapshot_url=go2rtc frame`。
- スキャン結果: `ipcamscan_*` テーブル＋`ScannedDevice`型（`src/ipcam_scan/types.rs`がSSoT）。SDM経路は「検出ではなくガイド」のみで登録はしない。
- OUI/RTSPテンプレ: `review_fixes_and_oui_rtsp_ssot_design.md` の SSoT（camera_brands / oui_entries / rtsp_templates / generic_rtsp_paths）。
- SDM設定: `settings.sdm_config`（UIはマスク返却）、環境原本 `/etc/is22/secrets/sdm.env` は初期投入用SSoTメモとして扱う。

## 1. 主要ギャップと統合方針
1) **Nestはサブネットスキャン非対応**  
　ポート/RTSP/ONVIFでは検出不可。IpcamScanは「候補提示＋SDM登録導線」のみを担う。

2) **Google OUI/ポート指紋の扱い**  
　Stage5スコアリングは `OUI=Google` の +20 だけだと誤登録を誘発する。`443/8443のみ開` かつ `RTSP/ONVIF無し` の場合はスコア0に近づけ、カテゴリは「要SDM」のサジェストに振る。

3) **結果UIの導線不足**  
　Camscan結果から SystemSettingsModal(SDMタブ)へ遷移するボタン/リンクが未定義。SuggestedActionを付与しUIに反映する。

4) **データ型差異**  
　`ScannedDevice` に SDM関連の示唆を載せるフィールドが無い。`suggested_action`/`discovery_path` を追加しSSoT化する。

5) **テスト網羅性**  
　IpcamScan修正（OUI/RTSP/カテゴリ）とSDM導線の組み合わせテストが未計画。異常系（OUIだけGoogle、RTSP無し、進捗表示、ボタン動作）を追加。

## 2. IpcamScan 側の設計変更（追加・補正）
### 2.1 型拡張（`src/ipcam_scan/types.rs`）
- 追加フィールド案（SSoTはこの型に一本化すること）:
  - `suggested_action: Option<SuggestedAction>`  
    - enum例: `None | Register | ManualCheck | SdmRegister | Ignore`
  - `discovery_path: DiscoveryPath`  
    - enum例: `SubnetScan | Manual | Sdm`（結果表示用）
  - `sdm_hint: Option<SdmHint>`  
    - `SdmHint { reason: String, enterprise_id?: String }`（理由をUI表示用に保持）

### 2.2 スコアリング/カテゴリ補正
- ルール追加:
  - `OUI=Google` かつ `open_ports ⊆ {443,8443,80}` で `rtsp_available=false` → `DeviceCategory=D (PossibleCamera)` ただし `suggested_action=SdmRegister`、スコアは 0〜10 に抑制（自動登録抑止）。
  - 上記条件で `category_detail` を `SdMOnly`（新設）に分岐させ、UI文言を固定する。
  - `CameraFamily::Nest` は IpcamScan検証対象から除外（rtsp_auth/onvif_authをスキップ）。

### 2.3 API追加/変更
- `/api/ipcamscan/devices` レスポンスに `suggested_action` / `sdm_hint` を追加。既存UIが無視しても後方互換を維持する。
- WebSocket進捗通知にも同項目を含める（進捗画面で「SDM登録に回ってください」を出せるように）。

### 2.4 UI連携（Scan Modal）
- Camscan結果行に「SDM登録へ」ボタン（`suggested_action==SdmRegister` のみ表示）。クリックで `SystemSettingsModal` を開き SDMタブを表示、`sdm_device_id` もしくは `oui_vendor=Google` のヒントを表示する。
- 進捗/カテゴリF表示では Nest を「通信断扱い」にしない（RTSPレスポンス前提のため）。LostConnection判定から Nest family を除外する。

## 3. GoogleDevices 側の整合アクション
- `GoogleDevices_introduction_BasicDesign`: IpcamScanで見つからないことを前提としつつ、`suggested_action=manual_check` の導線を「SDM登録」に名称統一する旨を追記する。
- `GoogleDevices_introduction_DetailedDesign`: `/api/sdm/config`/`/api/sdm/devices` に対し、Camscan UIからの導線（モーダル起動）を記載。`sdm_config` が未設定の場合に Camscan 側で出す警告文言を決める。
- `GoogleDevices_review.md`: 既出の「SuggestedActionをUIで出す」指摘を camscan仕様に反映したと明記する。

## 4. 実装タスク（MECE）
1. 型/DB: `ScannedDevice` 拡張 + `ipcamscan_devices` JSON列に `suggested_action`/`sdm_hint` を保存（マイグレーションが必要なら追加）。  
2. スコアリング: Google OUI + (443/8443) 判定ロジックの追加、スコア下げ・カテゴリ`SdMOnly`分岐。  
3. 検証ステージ: `CameraFamily::Nest` への `rtsp_auth/onvif_auth` 試行をスキップ。  
4. API: `/api/ipcamscan/devices` と WSイベントに新フィールドを出力。  
5. フロント: Scan結果UIに「SDM登録へ」導線、進捗/カテゴリ表示の文言追加、SystemSettingsModal起動連携。  
6. テスト:  
   - 単体: スコア計算・カテゴリ分岐（Google OUI/ポート条件で `SdmRegister` になる）。  
   - API: 新フィールドの後方互換と値検証。  
   - UI/E2E: Google OUIデバイス検出→ボタン押下で設定モーダルがSDMタブで開くこと、SDM未設定時の警告表示。  
   - 退行: 既存Tapo/VIGI検出が変化しないこと。

## 5. アンアンビギュアス確認
- 役割分担: IpcamScanは検出と導線提示のみ、SDM登録は `/api/sdm/*`。  
- トリガー条件: 「Google OUI + RTSP/ONVIF応答なし + 443/8443主体」で `suggested_action=SdmRegister` とする。  
- 導線: Scan結果UIから SystemSettingsModal(SDMタブ)を開く。  
- 後方互換: 既存 API/フィールドは保持、新フィールドは追加のみ。  
- テスト: 正常系/異常系/退行を列挙済み（上記5-テスト）。

## 6. 実装タイミングの注意
- **Camscan側修正が完了するまで、本メモに基づく導線やボタンのプレースホルダを実装しない。**  
- Camscan修正完了後に、本メモのタスクをIssue化して着手順序を決定する。現時点では設計のみ固定しコード変更は行わない。
