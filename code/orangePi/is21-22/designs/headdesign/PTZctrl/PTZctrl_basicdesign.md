# IS22 PTZ操作UI 基本設計書（LiveViewModal オーバーレイ方式 / 4方向）

> **更新日**: 2026-01-18
> **関連ドキュメント**: PTZctrl_SpecificationInvestigation.md, PTZctrl_BasicInstructions.md, PTZctrl_DetailedDesign.md

---

## 前提条件
- React+shadcn で実装・運用されている
- カメラタイル→LiveViewModalでライブ表示
- カメラ側にはPTZ能力フラグが既に存在
- 設定モーダルではPTZ能力表示まで実装済み

---

## 0. 目的

* カメラタイルをクリックして開く **LiveViewModal**（ライブ映像モーダル）上で、**PTZ対応カメラのみ**「崩スタ風の親指スティック（4方向）」を半透過オーバーレイ表示し、直感的にパン/チルト操作できるようにする。
* スマホ（タッチ）とPC（マウス/キーボード）で操作感を揃え、**チョン押し（最小入力）**と**長押し（連続移動）**の両方を提供する。

## 1. 現状（is22 実装の前提）

### 1.1 LiveViewModal（ライブ表示）

* カメラタイルクリックで開くモーダル。映像は go2rtc 経由のプレイヤーで表示 
* Video領域は `relative aspect-video bg-black` のコンテナで描画されている 
  → ここに `position:absolute` でPTZ UIを重ねるのが自然。

### 1.2 PTZ能力フラグ（すでにDB/型がある）

* `Camera` には `ptz_supported / ptz_continuous / ptz_absolute / ptz_relative / ptz_presets / ptz_home_supported` 等が既に定義されている 
* 仕様書（CameraDetailModal設計書）にも PTZ能力項目が明記 
* フロントの CameraDetailModal では PTZ能力が表示される（Badge）まで実装済み 

### 1.3 “モーダル同時利用制限”の仕組みが既に存在

* Backendに **Modal Lease** が実装されている（/api/modal/lease 等） 
* AdmissionController は「ユーザーが既に別モーダルを開いている場合は拒否」「TTL/Heartbeatで管理」 
  → PTZ操作も **“モーダルを開けている＝操作権がある”** に寄せると安全。

---

## 2. UX要件（操作感の設計）

## 2.1 チョン押し（最小入力）と長押し（連続）の考え方

PTZの最小動作はカメラ依存で、内部がステッピングモーターかどうかに関わらず「制御APIが許す最小変位/最小時間」に収束します。
そのためUIは以下の **2モードを統一的に提供**します：

* **Nudge（チョン押し）**：短い操作で “少しだけ動く”
* **Continuous（押してる間）**：操作している間ずっと動く

> 実装上は「相対移動が可能なら相対移動」「無理なら短時間の連続移動→自動STOP」で吸収するのが堅い（カメラ差をUI側で隠す）。

## 2.2 入力デバイス別の期待UX

### スマホ（タッチ）

* 親指スティックを **ドラッグ**して方向を指定（4方向固定）
* 指を離したら即停止（安全第一）

### PCブラウザ（マウス）

* **同じスティックをクリックしてドラッグ**（nipple系はマウスでも成立）
* 併設で **4方向ボタン（D-pad）**を置く（クリック長押しで連続 / クリックでnudge）

  * 理由：マウスはドラッグより「押しっぱなし」のほうがブレずに扱えるユーザーが多い

### キーボード（推奨）

* 矢印キー/WASD：押下で連続、短押しでnudge
* ESC：PTZ停止（＋モーダル閉じるは既存挙動に準拠）

---

## 3. UI仕様（表示/レイアウト）

## 3.1 表示条件

* `camera.ptz_supported === true` のときのみ表示 
* それ以外はPTZ UIを表示しない（誤操作防止）

## 3.2 配置（LiveViewModalへのオーバーレイ）

* LiveViewModal の Videoコンテナ（`relative aspect-video`）内に以下を重ねる 

  * 右下：ジョイスティック（メイン）
  * 左下：D-pad（任意。PC優先で表示/モバイルは省略可）
  * 右上：PTZ表示/ON-OFFトグル（誤タップ防止のため “折りたたみ” 可能にする）

## 3.3 タッチターゲット

* モバイルは最小 44x44px を担保（既存UIでも同様の指針あり ）

## 3.4 視覚表現（崩スタ風）

* 半透過 + ぼかし + 輪郭（ガラス/ホログラムっぽい）
* 押下中は「方向インジケータ（矢印）」が発光/強調
* 操作中のみ “MOVING” の小バッジを出す（誤操作の気づき）

---

## 4. 入力→PTZコマンド変換仕様（4方向限定）

## 4.1 方向の確定（4方向ゲート）

* スティックの角度を **up/down/left/right** に丸める
* デッドゾーン（中心）は無操作
* 方向が変わった時のみ “開始コマンド” を送る（連打しない）

## 4.2 Nudge（チョン押し）判定

* `pressDuration < TAP_THRESHOLD_MS` のとき Nudge とみなす

  * 例：`TAP_THRESHOLD_MS = 160ms`
* Nudgeは「小変位」or「短時間の連続移動」を実行し、自動停止

## 4.3 Continuous（長押し）判定

* `pressDuration >= TAP_THRESHOLD_MS` のとき Continuous
* 押している間は移動、離したら停止

## 4.4 安全停止条件（必須）

* 指/マウスボタンを離す
* モーダルを閉じる
* ブラウザが `blur`（タブ切替・ウィンドウ外）
* 方向が中心（デッドゾーン）に戻る

---

## 5. API仕様案（is22 backendに追加するPTZ API）

※ 現状 `routes.rs` には PTZ 操作エンドポイントは無いので 、追加仕様として定義します。

## 5.1 認可（Modal Lease連携：推奨）

* LiveViewModal表示時に `POST /api/modal/lease` で lease を取得 
* 以後の PTZ 操作は `lease_id` を必須にする
* AdmissionController は「1ユーザー1モーダル」制約とTTL/Heartbeatを既に持つ 
  → PTZの操作権限を “モーダルを開けているユーザー” に限定できる

### (A) Lease取得（既存）

* `POST /api/modal/lease` / `POST /api/modal/lease/:id/heartbeat` / `DELETE /api/modal/lease/:id` 

## 5.2 PTZ Move（新規）

### POST /api/cameras/:id/ptz/move

**Request**

```json
{
  "lease_id": "uuid",
  "direction": "up|down|left|right",
  "mode": "nudge|continuous",
  "speed": 0.0,
  "duration_ms": 120
}
```

* `mode=nudge` の場合 `duration_ms` を必須（サーバ側で自動STOP）
* `speed` は 0.0〜1.0（実機のレンジにマップ）

**Response**

```json
{ "ok": true }
```

## 5.3 PTZ Stop（新規）

### POST /api/cameras/:id/ptz/stop

**Request**

```json
{ "lease_id": "uuid" }
```

## 5.4 PTZ Preset/Home（将来 or 任意）

* `ptz_presets` や `ptz_home_supported` は既にモデル上あるため 、UI/APIは拡張しやすい

  * `POST /api/cameras/:id/ptz/home`
  * `POST /api/cameras/:id/ptz/presets/:name`

---

## 6. フロントエンド仕様（コンポーネント構成案）

is22 UIは React + shadcn が基盤である前提 

### 6.1 追加コンポーネント

* `frontend/src/components/ptz/PtzOverlay.tsx`

  * 表示条件：`camera.ptz_supported`
  * `LiveViewModal` の video領域内に absolute overlay
  * joystick +（任意で）D-pad + “PTZ”トグル + 状態表示
* `frontend/src/components/ptz/PtzStick4Way.tsx`

  * nipplejs をラップして `onMoveStart(dir)` / `onMoveStop()` を提供
* `frontend/src/components/ptz/PtzDpad.tsx`（任意）

  * shadcn Buttonで4方向
  * `onMouseDown` / `onMouseUp` / `onMouseLeave` で連続移動に向く

### 6.2 LiveViewModalの組み込みポイント

* 現状 LiveViewModal は Video領域を `div.relative.aspect-video` で囲っている 
  → ここに `<PtzOverlay />` を差し込む

---

## 7. 既存仕様との整合（is22 “世界観”）

* is22 は Camserver として「ブラウザUI（React+shadcn）」まで含む統合管理サーバ 
* すでに「モーダル＝ストリーム資源」という扱い（Lease/負荷制御）がある 
  → PTZもモーダルと同じスコープに閉じるのが安全で自然。

---

## 8. 受け入れ基準（アンアンビギュアス）

1. PTZ対応カメラでのみ、LiveViewModal上に半透過スティックが表示される
2. チョン押しで「小さく動く」、長押しで「押してる間動く」
3. 指を離す/blur/モーダル閉じるで必ず停止
4. PCは「ドラッグ」でも「D-pad長押し」でも操作できる
5. モーダルLeaseが取得できない（過負荷等）場合、PTZ UIは disabled 表示（誤操作不可）

---

## 9. “一般公開コンポーネントを使いたい”件の結論

* **見た目を崩スタ寄せ**しつつ運用しやすいのは、

  * **nipplejs（挙動） + shadcn（外枠/トグル/バッジ/ボタン）** のハイブリッド
    が一番「既存is22の設計思想（shadcn活用） 」と噛み合います。
* "React完成品ジョイスティック"も存在しますが、デザイン統一・イベント制御（nudge/stop保証）・保守性を考えると、is22では **薄いラッパー自作**が結局いちばん事故りにくいです。

---

## 10. 同時修正対象のバグ（PTZctrl_BasicInstructions.md参照）

### 10.1 LiveViewModalサイズ拡大
- **現状**: `max-w-4xl` (896px)
- **修正**: `max-w-5xl` (1024px) に拡大
- **理由**: 130%表示でも画像が小さく感じる

### 10.2 Rotation適用漏れ修正
| 対象コンポーネント | 現状 | 修正 |
|-------------------|------|------|
| CameraTile | transform未適用 | camera.rotationでtransform適用 |
| SuggestPane | transform未適用 | camera.rotationでtransform適用 |
| LiveViewModal | transform未適用 | camera.rotationでtransform適用 |
| CameraDetailModal | 適用済み | 変更なし |

### 10.3 SuggestPane 3台制限修正
- **問題**: 4台以上表示されることがある
- **原因**: exiting状態のカメラがカウントに含まれる
- **修正**: exiting除外後のカウントでslice実行

### 10.4 同一カメラ重複防止強化
- **問題**: 長時間動作で同一カメラが複数表示される
- **原因**: lacisIdチェックのタイミング問題
- **修正**: cameraIdベースの重複排除強化

### 10.5 onairTime制限厳密化
- **問題**: 設定されたonairTimeが守られないことがある
- **修正**: lastEventAt更新ロジックとexpiry判定の見直し

### 10.6 AccessAbsorber競合フィードバック追加
- **問題**: クリックモーダルとSuggestが競合時、理由が表示されない
- **修正**: AbsorberError.to_user_message()をフロントエンドで表示

---

## 11. 追加仕様（PTZctrl_BasicInstructions.md参照）

### 11.1 PTZ無効化チェックボックス
- **場所**: CameraDetailModal内PTZ能力セクション
- **DBフィールド**: `ptz_disabled BOOLEAN DEFAULT FALSE`
- **動作**: チェックONでLiveViewModalにPTZ UIを表示しない
- **PTZ非対応カメラ**: チェックボックスをグレーアウト

### 11.2 操作フィードバック強化
- 方向操作時に「← pan left」などの表示をカーソル近くに一時表示
- MOVINGバッジの明確な表示

### 11.3 Rotation追随
- PTZコントローラーの方向がカメラの回転設定に追随
- 例: 180度回転時、上方向押下で実際は下方向に移動

---

## 12. ファイル構成

### 新規作成
```
frontend/src/components/ptz/
├── PtzOverlay.tsx        # PTZ UIオーバーレイ（メインコンテナ）
├── PtzStick4Way.tsx      # 4方向ジョイスティック
├── PtzDpad.tsx           # D-padボタン（PC用）
└── index.ts              # エクスポート

frontend/src/hooks/
└── usePtzControl.ts      # PTZ操作カスタムフック

src/ptz_controller/
├── mod.rs                # モジュールエントリ
├── types.rs              # PTZ型定義
├── service.rs            # PTZサービス
└── onvif_client.rs       # ONVIF PTZクライアント

src/web_api/
└── ptz_routes.rs         # PTZ APIルート

migrations/
└── 032_ptz_disabled.sql  # ptz_disabledフィールド追加
```

### 修正対象
```
frontend/src/components/
├── LiveViewModal.tsx     # サイズ拡大、rotation、PTZ UI統合
├── CameraTile.tsx        # rotation適用
├── SuggestPane.tsx       # rotation、3台制限、重複防止、onairTime
├── CameraDetailModal.tsx # PTZ無効化チェックボックス追加

frontend/src/types/
└── api.ts                # ptz_disabled追加

src/config_store/
├── types.rs              # ptz_disabled追加
└── repository.rs         # UPDATE文更新

src/web_api/
├── mod.rs                # ptz_routesインポート
└── routes.rs             # ルーティング追加
```

---


