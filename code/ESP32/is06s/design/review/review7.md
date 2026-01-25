検証依頼書の内容（現象・再現条件・優先順位）はかなり良いです。特に **Digital/PWMどちらでも1.9V固定** まで押さえているのが強いです  。
そのうえで、いまの情報から **最優先で追加したい“濃厚仮説”** が1つあります。

---

## 最有力仮説：Arduino-ESP32 3.x のLEDC API誤用で「CH1だけ未初期化 or 書き込み先がズレている」

IS06S側の実装は `initLedc()` で

* `bool ok = ledcAttach(pin, freq, resolution);`
* `ok` が true のときだけ `ledcWrite(pin, 0);`

という形になっています 。
しかし、同リポ内の **is06a_spec** では Arduino-ESP32 3.x の前提として、

* `int ch = ledcAttach(pin, freq, 8);`（戻り値は **チャンネル番号**）
* デューティは `ledcWriteChannel(ch, duty)` を使う
* **注意：`ledcWrite(pin, duty)` は非推奨。必ず `ledcWriteChannel(ch, duty)`** 

と明記されています。

ここから逆算すると、いまの IS06S の書き方だと典型的にこう崩れます：

* `ledcAttach()` が **ch=0** を返す（最初の確保）
  → `bool ok = 0` になって **CH1だけ “FAILED扱い”** になり、初期 `ledcWrite(...,0)` が実行されない 
  → その結果、CH1が“謎のデューティ（≒中途半端）”のまま残り、D/Pどちらでも **1.9V付近に見える**（あなたの計算どおり約58%相当） 
* さらに `ledcWrite(pin, duty)` 自体が非推奨で、期待どおり pin 書き込みにならず **チャンネル/紐付けズレ** を起こす可能性 

「周波数 5kHz→20kHz で発生」に見えても、実態は **“LEDCの扱い（戻り値/書き込み先）”の不整合が顕在化した** 可能性が高いです（周波数はトリガーで、根本はAPIの解釈違い）。

---

## 追加で危ない点：`setPinType()` でも `ledcAttach()` を再実行している

`setPinType()` 側でも D/P チャンネルなら `ledcAttach(...)` を呼び直しています 。
これを **チャンネル管理なし** で繰り返すと、確保リーク/紐付けズレ/意図しないチャンネル再割当が起こりやすいです（CH1だけ挙動が変、の説明にもなります）。

---

## すぐ効く修正方針（is06a流に統一）

is06a_spec の方針  を IS06S に適用するのが最短です。

### 1) `initLedc()`：戻り値を `int ch` で受け、chを保持

* `bool ok` をやめる（**ch=0 を失敗扱いしない**）
* `ledcWrite(pin, …)` をやめ、`ledcWriteChannel(ch, …)` に統一
* `ch < 0` ならフォールバックで pinMode OUTPUT + LOW（浮き対策）

### 2) `applyDigitalOutput()/applyPwmOutput()`：必ず “ch指定” で書く

* Digital: duty = 0 / maxDuty
* PWM: value(0–100) → duty(0–maxDuty) にマップ
* maxDuty は `((1 << PWM_RESOLUTION) - 1)`（いま 8bit なら 255）
  ※設定は `PWM_FREQUENCY=20000`, `PWM_RESOLUTION=8` 

### 3) `setPinType()`：再 attach するなら「detach→attach→ch更新」までやる

いまは attach だけしてch管理が無いので 、修正後の構造に合わせて更新必須です。

---

## いまの検証優先事項に“1個だけ追加”するならこれ

依頼書の優先事項（シリアルログ／周波数戻し／CH2-4確認）は正しいです 。
そこに **「ledcAttach の戻り値（ch番号）をログに出す」** を入れると決定打になります。

ログ例（イメージ）：

* `CH1 GPIO18: ledcAttach -> ch=0`
* `CH2 GPIO5:  -> ch=1`
* `CH3 GPIO15: -> ch=2`
* `CH4 GPIO27: -> ch=3`

もしこの並びが出たら、現状コードの `bool ok` が **CH1だけ誤判定していた** と確定します。

---

## Issue#123 に貼れるコメント案（コピペ用）

* 最有力：Arduino-ESP32 3.x LEDC APIの誤用。`ledcAttach()` は bool ではなく **ch(int)** を返し、**ch=0 が正当**。現状 `bool ok = ledcAttach(...)` だと CH1 が 0→false になり “FAILED扱い” で初期 `ledcWrite(...,0)` がスキップされる可能性が高い（結果、CH1が中途半端Dutyで固定→1.9V相当）。
* さらに is06a_spec にある通り **`ledcWrite(pin,duty)` は非推奨で、`ledcWriteChannel(ch,duty)` に統一すべき**。
* 対応方針：initLedcで `int ch=ledcAttach(...)` を保存、書き込みは `ledcWriteChannel(ch, ...)`、`ch<0` は pinMode OUTPUT + LOW のフォールバック。`setPinType` 側の再attachも detach→attach→ch更新まで含めて整理。

---

