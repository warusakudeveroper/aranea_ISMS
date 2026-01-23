了解。**「同一ハードで別FWが安定」＝“電源電圧降下”は前提から外す**として、残りは **(1) リセットライン（EN）を誰かが叩いている** か **(2) このFW固有の初期化/処理が内部破壊→結果としてリセット** のどちらかに寄せて切ります。

結論から言うと、このリポの現状コードだと **“ブート直後に必ず走る処理”が2つ**あって、そこが最優先の切り分けポイントです。

---

## まず最優先で疑う2点（このFW固有で、ブート直後に必ず走る）

### 1) StateReporter が「初回ループで必ず HTTP 送信」する

`StateReporterIs06s::update()` は `lastHeartbeatTime_ == 0` の場合に即 `sendHeartbeat()` を叩きます。
`sendStateReport()` の最後で `Done: %d success` を出すので、あなたの “`e: 1 success` 連打” はここ由来で説明がつきます。

**切り分け最短**：`loop()` の `stateReporter.update();` を一旦コメントアウト（または heartbeatInterval を 0 に）して、クラッシュループが止まるか確認。

---

### 2) Is06PinManager が「begin() で必ず LEDC attach + ledcWrite」を4本分やる

`Is06PinManager::begin()` は `initLedc()` を必ず呼び、4ch分 `ledcAttach(...)` の後に **`ledcWrite(IS06_PIN_MAP[i], 0)`** を叩きます。
さらに出力適用でも `ledcWrite(IS06_PIN_MAP[idx], pwm)` を使っています。

ここ、Arduino-ESP32 3.x の LEDC API の解釈がズレていると**即死級**になり得ます（別FWが安定でも、このFWだけがここを踏むなら説明可能）。

**切り分け最短**：`Is06PinManager::begin()` の `initLedc();` を一旦外してビルドし、クラッシュループが止まるか確認。
（※ `setPinType(...PWM_OUTPUT)` 側でも `ledcAttach` してるので、少なくとも“常時 attach”は削っても機能的に破綻しにくい構造です。）

---

## 「シリアル出力が怪しい」線の最短確認

“Serial print がメモリ干渉”というより、**PC側が DTR/RTS を頻繁にトグルして EN を叩いてる**可能性のほうが現実的です（別FWだとPC側ツールの挙動が変わって出ないことはあります）。

**最短テスト**：

* USB電源はそのままでもいいので、**データ線を切る（データブロッカー/充電専用ケーブル）**状態で動かして、リブートが止まるか確認。

  * 止まる → PC/シリアル制御線起因（DTR/RTS）
  * 止まらない → FW内部問題に絞れる

---

## 5分で終わる “二分探索” 手順（おすすめ順）

1. **StateReporter停止**（`stateReporter.update()` を外す）→安定するか
2. まだ落ちるなら **PinManager停止**（`pinManager.update()` を外す）→安定するか
3. まだ落ちるなら **PinManager の initLedc を外す**（上記）
4. それでも落ちるなら HttpManager停止（`httpMgr.handle()` を外す）

この4つで「どの層がトリガーか」は必ず特定できます（MQTT無効でも落ちるので、今はこの4点だけで十分）。

---

## 特定後の“直し方”の当たり（現時点での推し）

* **StateReporter起因なら**：初回ハートビートを遅延させる（例：`begin()`で `lastHeartbeatTime_ = millis();` にして「起動直後は送らない」）が安全・効果大。
* **PinManager/LEDC起因なら**：

  * `initLedc()` を廃止して **PWM_OUTPUT のチャンネルだけ attach**（NVS読んだ後に必要なものだけ）
  * もしくは `ledcWrite` が “pin指定” で正しいかを確実化して、ズレるなら “channel指定” の書き方に統一（ここが一番事故りやすい）

---


