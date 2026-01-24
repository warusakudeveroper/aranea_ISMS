以下、コミット **025146fcf750e16285f63b47c44be8d2fff4edd4**（+822/-246, 6 files）を「実装としての正しさ」「設計適合」「運用リスク」「ドキュメント整合」の観点で、かなり踏み込んでレビューします。
（※実機/ビルドは回せていないので、コードと仕様文面からの静的レビューです）

---

## 0. まず結論（このコミットの評価）

やっていること自体は **P1-1〜P1-5 を実装に落とし込む方向性としては正しい** です。特に：

* `POST /api/pin/{ch}/setting` の TODO を潰して、**API→PinManager→NVS** の経路を通したのは良い
* `stateName` を **UI表示（PIN Control）まで通した** のは大きい
* NVSキーを短縮・統一（suffix定義）して、ESP32 NVSの制約を踏んだのも良い

ただし、現状のままだと **「完了報告・乖離0%」と言い切るには危険な“穴”が複数あります**。
特に致命度が高いのは次の2点です：

1. **PIN Settingsタブが “現在値を正しく表示できない” 可能性が高い**（APIレスポンス設計ミス）
2. ドキュメント（GAP_ANALYSIS / IMPLEMENTATION_REPORT）に **事実と違う記述** が混ざっていて、レビューや運用を誤誘導する

---

## 1. ファイル別レビュー（何が入ったか／何が良いか／何が危ないか）

### A) `code/ESP32/is06s/AraneaSettingsDefaults.h`

**良い点**

* NVS suffix を定義してキーを短縮（例: `_val`, `_deb`, `_roc`, `_stn`, `_alloc`）
* ESP32 NVSのキー長制約に対して安全な設計（`ch1_expDt` 等は十分短い）

**気になる点（軽）**

* 既存コード側にまだ `"_en"`, `"_type"` の直書きが残っていて、統一感が崩れてます（動作は同じなので軽微）

---

### B) `code/ESP32/is06s/Is06PinManager.h / .cpp`

#### 追加された機能の評価

* `setActionMode / setValidity / setDebounce / setRateOfChange / setName / setAllocation / setStateName` を追加し、各々で **NVS永続化**まで実装 → 方針は良い
* `loadFromNvs()` が name/mode/val/deb/roc/allocation/stateName を読むようになった → 起動後復元OK
* `allocation` を CSV、`stateName` を JSON配列文字列として保存 → 低コストで妥当

#### ⚠️ 重要な問題 1：Digital Output “Alt(トグル)” の debounce が効かない

`setPinState()` の Digital Output 側で、

* **Mom** のときは `debounceEnd = now + getEffectiveDebounce(channel)` を設定している
* **Alt**（else側）は **debounceEnd を更新していない**

つまり Alt モードのとき、`now < debounceEnd` チェックがほぼ機能せず、**チャタリング/連打抑止が効かない**状態になりやすいです。
DesignerInstructions でも “Altでも必要に応じてDebounce” のニュアンスがあるので、ここは修正推奨（高優先）。

> 対応案：Alt側でも state変化後に `debounceEnd = now + getEffectiveDebounce(channel)` を入れる

#### ⚠️ 重要な問題 2：ActionMode と PinType の整合バリデーションがない

例えば：

* pin type を PWM にしたのに actionMode が `MOMENTARY` のまま、などが普通に起きうる

  * 現状 PWM遷移は `actionMode == RAPID` 以外は “Slow扱い” になるので、動きはするが **仕様的に混乱**します
* Digital Output なのに actionMode が `SLOW/RAPID/ROTATE` の場合、コード上は `MOMENTARY以外=Alt扱い` でトグルになります（意図しない動作）

> 対応案：
>
> * `setPinType()` 内で **typeに応じて actionMode をデフォルトへ補正**
> * `setActionMode()` 内で **typeに対して許可されない mode を弾く/補正する**

#### ⚠️ 中くらい：`saveToNvs()` が “全設定保存” を名乗る割に不完全

`saveToNvs()` は name/mode/val/deb/roc/allocation/stateName だけ保存していますが、

* enabled/type/expiryDate/expiryEnabled などは保存しません（別setterに依存）

実運用で `setPinSetting()` を使うと、**一部フィールドが永続化されない**ことになります。
今後 API で “まとめて1回保存” 方向へ寄せるなら、ここは整理しておくと事故が減ります。

#### ⚠️ 中くらい：stateName JSON の生成/パースが素朴

* 生成は `"` のエスケープ無し（`\"` が入ったら壊れる）
* パースもエスケープ未対応

現状 UI の想定入力がシンプルなら許容ですが、将来 “名前に記号” を入れると壊れがちです。
（ArduinoJsonで配列として serialize/deserialize すると安全）

---

### C) `code/ESP32/is06s/HttpManagerIs06s.cpp`

#### 良い点

* `PIN Control` の表示が `stateName` と `name` に対応（Digital/PWM/Input）
* `savePinSettings()` が “全CHにPOST” を実装している（P1-1の中心）

#### ⚠️ 重要な問題 1：`/api/pin/all` が “設定値” を返していない

`loadPinStates()` と `loadPinSettings()` がどちらも `/api/pin/all` を叩きます。

しかし `/api/pin/all` は `buildAllPinsJson()` → `buildPinStateJson()` の結果で、現状含んでいるのは主に：

* `channel, enabled, state, pwm, name, type, actionMode, stateName`

**ここに `validity / debounce / rateOfChange / allocation / expiry…` が入っていません。**
そのため `PIN Settings` タブは **「現在値を正しく表示できない」** 可能性が高いです。

実際 JS 側は `p.validity || 0` のように書いてあるので、

* 未返却なら全部 0 表示
* ユーザーがそのまま Save All を押すと、**意図せず 0 を上書き**する危険がある

> 対応案（おすすめ順）
>
> 1. `/api/pin/all` を **“state+settingの統合レスポンス”** にする（設定タブも制御タブもこれ1本）
> 2. Settingsタブだけ `/api/pin/{ch}/setting` を6回叩いて埋める
> 3. `buildPinStateJson` に `validity/debounce/roc` だけでも追加（最低限の暫定）

#### ⚠️ 重要な問題 2：「Save All Settings」が “All” ではない

`savePinSettings()` が送っているのは：

* `type, name, actionMode, validity, debounce, rateOfChange, stateName`

**allocation / expiryDate / expiryEnabled / enabled** は送っていません。
UIにも入力欄がありません。
にも関わらず docs では “全設定保存” や “allocation/expiry UI完了” と書いているので、ここは齟齬です。

#### ⚠️ 中くらい：UI値のエスケープ無し（壊れやすい）

`<input value='...'>` に name/stateName をそのまま入れているので、

* `'` や `"` を含む名前で HTML が壊れる
* 最悪、ローカルUIでも注入が起こり得る

> 対応案：最低限、`& < > " '` を HTMLエスケープするヘルパを JS 側に入れる

#### ⚠️ 軽：PWMラベル更新が stateName を無視

PWMスライダー変更後、ラベルを常に `value + '%'` にしているので、stateNameラベルが一時的に消えます。
（次回 `loadPinStates()` で再描画されるので致命ではないですがUXとしては惜しい）

---

### D) `design/GAP_ANALYSIS.md` / `design/IMPLEMENTATION_REPORT.md`

ここは **今回いちばん危ない** です。

* 「`/api/pin/all` は全PIN状態・設定取得 ✅」
  → 実装は現状 **“状態寄り”** で、設定値（validity等）が入っていない
* 「allocation / expiryDate 設定UI ✅」
  → UIに該当入力欄がない（少なくとも `HttpManagerIs06s.cpp` の JS には無い）
* 「triggerType 連動先から推論 ✅」
  → コード上に triggerType フィールド/ロジックは見当たらない（少なくとも Is06PinManager には無い）

**“完了” と書くことでレビュー観点がズレて、本番で事故るタイプの齟齬**です。
この2ドキュメントは、コードが追いつくか、記述を「API対応のみ」などに落として整合させるのが必須です。

---

## 2. P1-1〜P1-5 要件レビュー（達成度）

### ✅ P1-2 / P1-4 はかなり良い（ただし型整合のバリデーションは欲しい）

* stateName の保存→NVS→API→UI表示までつながった
* actionMode の保存/読込（NVS）もつながった

### ⚠️ P1-1 / P1-5 は「実装したが、現状だと運用で壊れる可能性」

* SaveAll の動作はあるが、**初期表示値が正しく入らない**ため、Saveが上書き事故を起こし得る
* validity/debounce/rateOfChange は API/保存できるが、**UIが現値を出せない**可能性が高い

### ⚠️ P1-3 は「API/永続化はOK、UIは未達」

* allocation 自体は API で入るし NVS もある
* ただし設計書の “バリデーション（同一タイプのみ・混在禁止）” は未実装

---

## 3. “Must Fix（先に潰すべき）” リスト

優先度順に並べます。

1. **/api/pin/all のレスポンスを Settings タブが必要とする形にする**

   * これが直らない限り「SaveAll」は事故りやすい

2. **Alt(トグル) の debounce を効かせる**（Digital Output）

   * 現場のスイッチ/チャタリング系でハマりやすい

3. **ドキュメントの事実誤認を修正**（特に `/api/pin/all` と UI 対応範囲）

   * レビューと運用判断を誤らせる

---

## 4. “Should Fix（品質を上げる）” リスト

* PinType×ActionMode の整合チェック（補正 or reject）
* allocation のバリデーション（同一タイプ縛り、混在禁止、存在しないCH拒否）
* stateName JSON を ArduinoJson で安全に serialize/deserialize
* UIのHTMLエスケープ（名前に `'` が入っただけで壊れるのは普通に起きる）
* SaveAll の対象を明確化（本当にAllにするか、ボタン名を変える）

---

## 5. 最小の動作確認シナリオ（レビュー観点で“ここだけは”）

実機でやるなら、最低でもこれ：

1. **初期表示が正しいか**

   * `/api/pin/1/setting` で `validity/debounce/roc` を入れる
   * PIN Settings タブを開いて、その値がフォームに反映されるか
     → ここが現状一番怪しい（/api/pin/all問題）

2. **Alt の debounce**

   * CH1 を digitalOutput + Alt
   * 連打（もしくはMQTT連打）で debounce が効いて “reject” になるか

3. **stateName 表示**

   * digitalOutput: `["on:解錠","off:施錠"]` → ボタンラベルが切り替わるか
   * digitalInput: `["on:OPEN","off:CLOSE"]` → 表示が変わるか
   * pwmOutput: `["0:消灯","30:弱","60:中","100:強"]` → 0/30/60/100 の時だけラベルが出るか

---

