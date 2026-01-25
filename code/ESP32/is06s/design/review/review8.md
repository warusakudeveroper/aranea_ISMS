# 検証報告


---

## 1) is06s コードベース内の「SPI使用」実態（静的スキャン結果）

### is06s.ino（メインスケッチ）

`#include` 群に **SPI系は出てきません**（Wire/WiFi/HTTPClient/ArduinoJson/esp_mac + AraneaGlobalImporter） 
→ 少なくとも **is06s.ino 直書きで SPI を使っている形跡は無し**。

### is06s/AraneaGlobalImporter.h（is06s が取り込む共通モジュール束）

is06s用の importer は `SettingManager/WiFiManager/NtpManager/.../IOController` 等を include していますが、ここにも **SPI の include 指定はありません** 
→ **「importer経由でSPI.hを明示的に引いている」線も薄い**。

### リポジトリ横断のキーワード検索（`#include <SPI.h>` / `SPI.begin` / `spi_bus_initialize` / VSPI/HSPI）

こちらも **is06s 実コード側でヒットしていません**（ヒットするのはドキュメント上の “VSPI SCK” 言及など） 
※ `code/ESP32/is05a/build/.../SPI.cpp.d` のような **ビルド生成物**にはSPIが出ますが、これは *is05a のビルド成果物*なので “is06s がSPIを使っている根拠” にはなりません 

✅ **結論（静的検証）：is06s のコードとしては「SPIを呼んでいる証拠は見つからない」**
→ なので、次は「**“暗黙にSPIが動いてる” をコードで証明/否定**」のフェーズに入るのが合理的です。

---

## 2) is06s の GPIO18/5 を“奪っている可能性”のあるコード経路（静的に確定できる範囲）

### (A) is06s.ino の initGpio() が CH1-4 を OUTPUT LOW にする

GPIO18/5 を含む CH1-4 を `pinMode(OUTPUT)`→`digitalWrite(LOW)` しています 
→ ここは「出力を“握る”」動き。

### (B) Is06PinManager が setup 後に LEDC を全CH1-4へ attach して、以降は ledcWrite(pin, duty) で駆動

`begin()` → `initLedc()` で CH1-4 全てに `ledcAttach(pin,...)` 
`applyDigitalOutput()` / `applyPwmOutput()` は CH1-4 を **常に ledcWrite(pin, …)** で書きます 
さらに `setPinType()` でも CH1-4 に対して `ledcAttach()` を再実行しています 

👉 つまり **GPIO18/5 は「最終的にLEDC側で握る」設計**です。
なので、あなたの「gpio_get_level() は 1 を返すが実測電圧が低い」という現象は、**“物理的にPWM的な波形（高周波トグル）になっていてテスターが平均値を見ている”**ケースでも成立します（この場合 `gpio_get_level()` は瞬間値で 1 を返し得ます）。

---

## 3) 「SPIを徹底的に禁止」するための対策（コードで強制する）

### 対策1：is06s だけ “SPI.h を絶対に通さない” （強制コンパイルエラー）

`code/ESP32/is06s/` 直下に **ローカルの `SPI.h` を置いて潰す**のが一番強いです。

```cpp
// code/ESP32/is06s/SPI.h
#pragma once
#error "SPI is forbidden in IS06S firmware. Do not include SPI.h (directly or indirectly)."
```

Arduino/PlatformIO の include path では、スケッチ/ローカルが先に解決される構成が多いので、**誰かが間違って `#include <SPI.h>` しても即ビルドが落ちます**。
（もし環境で優先順位が逆なら、platformio.ini / arduino-cli の include path を調整してこのディレクトリを先頭に寄せます）

### 対策2：CIで “SPI痕跡” を検知して落とす（人間ミス対策）

GitHub Actions で、is06s 配下＋global共通に対して禁止語を grep して PR を落とすのが安定です。

* 禁止例：`#include <SPI.h>` / `SPI.begin` / `SPIClass` / `spi_bus_initialize` / `VSPI` / `HSPI`

（あなたの運用だと *is06s だけ禁止*で十分なら、検索対象を `code/ESP32/is06s` に絞る）

### 対策3：ランタイムで “SPI2/SPI3(=HSPI/VSPI相当)” を disable する（保険）

Arduino-ESP32上でも ESP-IDF の peripheral disable を使えることがあります。
「SPIを絶対に使わせない」思想なら、**setupの早期に SPI2/SPI3 を disable**しておくのはアリです（※SPI0/1＝フラッシュ周りは触らない）。

---

## 4) “暗黙SPI/別ペリフェラルが18/5を奪ってるか” を **コードだけで確定**する方法

実機切り分け（最小スケッチ）が無理なら、**ファームに「証拠採取用の計測API」を入れる**のが最短です。

### 方法A：GPIO18/5 の「瞬間サンプリング」で duty を推定（PWM/トグル検出）

`gpio_get_level()` を短時間で大量サンプルして High率を出す。

* High率が 1.0 に近い → ほぼDC High（テスター低電圧なら外部で電圧が落ちてる可能性）
* High率が 0.3〜0.6 など → **何かが高速トグルしてる**（LEDC/SPI/別信号）

これを **HttpManagerIs06s に `/api/debug/gpio` 的なエンドポイント**で返せば、USB不要で採取できます。

### 方法B：GPIOの “出力セレクタ” レジスタをダンプ（どの周辺回路が駆動してるか）

ESP32 は GPIO Matrix を使うので、`GPIO.func_out_sel_cfg[pin]` 系を読むと「そのピンが何の信号に繋がってるか」を確認できます。
これを 18/5/15/27 でダンプして比較すれば、**CH1/CH2 だけ別信号に割り当たってる**などが即判明します。

---

## いま私が言い切れること（コード検証の暫定結論）

* is06s のソース上は **SPIを明示的に使っている痕跡は見つからない**（is06s.ino/importerともにSPI無し）
* 一方で CH1-4 は最終的に LEDC で握る設計で、`ledcAttach`/`ledcWrite` が複数箇所から呼ばれるため、**「CH1/CH2だけ“何かの波形”が出て平均電圧が低い」状況はコード的に十分起こり得る**
* だから次は、**“GPIOがトグルしてるのか/DCなのか/何の信号に割当たってるのか”をHTTP経由で採取**して、暗黙SPIかLEDCか他ペリフェラルかを確定させるのが最短です。

---

# 追加検証

ESP32は **GPIOマトリクス（内部信号→ピンの配線）**を持っていて、**LEDC（PWM）やSPIなどの周辺回路がそのピンに“出力信号を割り当てたまま”**だと、あなたが `pinMode(pin, OUTPUT)` / `digitalWrite(pin, HIGH)` を呼んでも、**「ピンの実波形」は周辺回路側が勝つ**ことが起こりえます。
その結果、

* コード上は「OUTPUT/HIGH」っぽい
* `gpio_get_level()` も 1 を返すことがある
* でも実測は 1.3V台（平均値）/ MOSFETが中途半端

という矛盾が成立します。

ここから先は「コードベースだけで徹底的に潰す」ための、**手順を固定した検証指示**です（配線変更ゼロ前提）。

---

## 0) まず“何が勝ってるか”を可視化する（Pin Auditを入れる）

ESP-IDFには、**現在そのピンが入力/出力か、プル設定、OpenDrain、そしてGPIOマトリクスで何の信号が出力に割り当たっているか**をダンプする `gpio_dump_io_configuration()` があります。公式ドキュメントにも例があり、`GPIO Matrix SigOut ID: 256 (simple GPIO output)` なら「単純GPIO出力」です。([Espressif Systems][1])

これを**あなたの本番ファームに埋め込んで**、どのタイミングで GPIO18/5 が奪われるかを特定します。

### 0-1) そのまま貼れる監査コード（Arduino-ESP32想定）

`PinAudit.h` みたいなファイルを1つ作って、以下を入れてください。

```cpp
#pragma once
#include <Arduino.h>
#include <stdio.h>
#include "driver/gpio.h"

// CH1/CH2
static constexpr uint8_t PIN_CH1 = 18;
static constexpr uint8_t PIN_CH2 = 5;

static inline void dumpPins(const char* tag) {
  Serial.printf("\n\n===== PIN AUDIT: %s =====\n", tag);

  // 18/5 のGPIO設定と、GPIOマトリクスの割り当て状態をダンプ
  gpio_dump_io_configuration(stdout, (1ULL << PIN_CH1) | (1ULL << PIN_CH2));

  // LEDC(PWM)に“紐付いているか”を追加で確認
  // ※ Arduino-ESP32 のLEDC API: ledcRead / ledcReadFreq が使える想定
  Serial.printf("LEDC: CH1(pin %u) freq=%u duty=%u\n", PIN_CH1, ledcReadFreq(PIN_CH1), ledcRead(PIN_CH1));
  Serial.printf("LEDC: CH2(pin %u) freq=%u duty=%u\n", PIN_CH2, ledcReadFreq(PIN_CH2), ledcRead(PIN_CH2));

  // ついでに論理値（参考）
  Serial.printf("digitalRead: CH1=%d CH2=%d\n", digitalRead(PIN_CH1), digitalRead(PIN_CH2));
}
```

* `gpio_dump_io_configuration()` は **ピンのマッピングまで見える**のが強みです（どの周辺信号が刺さっているか）。([Espressif Systems][1])
* `ledcReadFreq/ledcRead` は、そのピンがLEDCの管理下にある気配を掴む用途です（LEDC API仕様はArduino公式ドキュメント）。([Espressif Systems][2])

> もし `gpio_dump_io_configuration` が「見つからない/宣言されてない」でコンパイル落ちする場合：
> Arduinoコア/IDFの組み合わせ問題の可能性があるので、その場合は代替のレジスタ読みに切り替える必要があります（その時はコンパイルエラー文をそのまま貼ってください。次の指示を出します）。

### 0-2) 監査を差し込むポイント（超重要）

あなたの `setup()` を「フェーズ分け」して、それぞれの直後に `dumpPins()` を入れます。

例：

```cpp
#include "PinAudit.h"

void setup() {
  Serial.begin(115200);
  delay(500);

  dumpPins("boot (just after Serial)");

  // (A) あなたが“正しいはず”と考えるCH1/CH2初期化直後
  pinMode(PIN_CH1, OUTPUT);
  pinMode(PIN_CH2, OUTPUT);
  digitalWrite(PIN_CH1, HIGH);
  digitalWrite(PIN_CH2, HIGH);
  dumpPins("after CH1/CH2 set OUTPUT HIGH");

  // (B) WiFi開始直後
  // WiFi.begin(...);
  dumpPins("after WiFi.begin");

  // (C) FS/ストレージ初期化直後（SPIFFS/SD/SD_MMC等）
  // SPIFFS.begin();
  // SD.begin();
  dumpPins("after FS init");

  // (D) WebServer/AsyncWebServer開始直後
  // server.begin();
  dumpPins("after webserver begin");

  // (E) “周辺機器ライブラリ begin()” の直後（TFT/センサ/何か）
  // xxx.begin();
  dumpPins("after device begin");

  // (F) タスク生成直後（xTaskCreate等）
  // xTaskCreate(...);
  dumpPins("after tasks created");
}

void loop() {
  static uint32_t t = 0;
  if (millis() - t > 3000) { // 3秒に1回だけ
    t = millis();
    dumpPins("periodic");
  }
}
```

**狙い**：
「最初はSigOut=256（普通のGPIO）だったのに、ある地点から違うIDになる」
→ **“その直前に実行した何か”が犯人**です。

---

## 1) ダンプ結果の読み方（ここだけ覚えればOK）

ESP-IDFドキュメント例では、GPIO出力の正常例として：

* `OutputEn: 1`
* `FuncSel: 1 (GPIO)`
* `GPIO Matrix SigOut ID: 256 (simple GPIO output)`

が出ます。([Espressif Systems][1])

あなたの監査ログで見るべきはこれです：

### ✅ 正常（あなたが欲しい状態）

* `OutputEn: 1`
* `OpenDrain: 0`（基本これ。1ならHが弱くなる可能性）
* `GPIO Matrix SigOut ID: 256 (simple GPIO output)` ([Espressif Systems][1])

### ❌ 典型的な“奪われてる”状態

* `GPIO Matrix SigOut ID` が **256以外**
  → そのピンは「単純GPIO」ではなく、何か周辺回路の信号が刺さってます。([Espressif Systems][1])

### ❌ そもそも出力になってない

* `OutputEn: 0`
  → HIGHにしてもHi-Zで、外部の分圧やリークで1.3V台みたいな “中間電圧” になり得ます。

---

## 2) 次にやる“静的コード検証”（grepで犯人候補を全列挙）

ここからが「徹底したコードベース検証」です。
目的は **GPIO18/5 に関係しうるAPI呼び出しを100%洗い出す**こと。

以下は ripgrep (`rg`) 前提で書きます（Linux/macOS/WindowsのGit BashでもOK）。

### 2-1) まず“直書き”を全部拾う（18/5/CH1/CH2）

```sh
rg -n "GPIO_NUM_18|GPIO_NUM_5|\b18\b|\b5\b|CH1|CH2" .
```

* 数字の `5` はノイズが多いので、後で絞ります。

次に **pinMode/digitalWrite/LEDC系**に限定：

```sh
rg -n "pinMode\s*\(\s*(18|5)\s*,|digitalWrite\s*\(\s*(18|5)\s*,|gpio_set_direction\s*\(\s*GPIO_NUM_(18|5)|gpio_reset_pin\s*\(\s*GPIO_NUM_(18|5)" .
```

### 2-2) PWM系（LEDC/analogWrite/tone/Servo）を全列挙

Arduino-ESP32のLEDC APIは `ledcAttach/ledcWrite/ledcDetach` などが公式です。([Espressif Systems][2])
`analogWrite()` もLEDC経由でPWMです。([Espressif Systems][2])

検索：

```sh
rg -n "\bledc(Attach|AttachChannel|Write|WriteChannel|Detach|Read|ReadFreq|WriteTone|WriteNote|Fade)\b" .
rg -n "\banalogWrite(Resolution|Frequency)?\s*\(" .
rg -n "\btone\s*\(" .
rg -n "Servo|ESP32Servo|writeMicroseconds|attach\s*\(" .
```

**ここで1箇所でも引っかかったら**、Pin Auditの結果と突き合わせてください。
（例：`analogWrite(18, 100)` とかあると、デューティ比由来の平均電圧になっても不思議じゃないです）

> なお Arduino-ESP32 3.0移行でLEDC APIが変更され、`ledcSetup`/`ledcAttachPin` は削除、`ledcDetachPin` は `ledcDetach` に改名されています。古いAPIがコード/依存ライブラリに残っていると、意図せずLEDCが絡む構成になりがちです。([Espressif Systems][3])

### 2-3) SPI系（最重要：GPIO18/5はVSPIデフォルトのSCLK/SS）

Arduino-ESP32公式のSPI Multiple Buses例に **VSPIのデフォルトが SCLK=18, SS=5** と明記されています。([Espressif Systems][4])
つまり、コードベースに `SPI.begin()`（引数なし）や `SD.begin()`（内部でSPI）等があるだけで、**CH1/CH2がSPIに奪われる**可能性が高いです。

検索（徹底）：

```sh
rg -n "\bSPI\.begin\b|SPIClass|VSPI|HSPI|\bSD\.begin\b|\bSD_MMC\.begin\b" .
rg -n "\b(SCK|SS|MOSI|MISO)\b" .
```

ポイント：

* `SCK` / `SS` を使っているコードは、ボード定義によって **結果的に18/5** を指します（公式例でVSPIデフォルトがそう）。([Espressif Systems][4])
* ライブラリが内部で `SPI.begin()` を呼んでいるケースが多いので、**あなたのsrcだけでなく libdeps / libraries も対象に**検索してください。

---

## 3) “犯行タイミング”が掴めたら、次はコードだけで切り分け（二分探索）

Pin Auditで「ここでSigOut IDが変わった」という地点が見つかったら、そこからは機械的にやります。

### 3-1) begin() 単位でコメントアウト（再配線不要）

たとえば「after FS init」で変わったなら、FS周りをこう分割：

* `SPIFFS.begin()` の直後にdump
* `SD.begin()` の直後にdump
* `Preferences.begin()` の直後にdump
  …という風に、**“呼んだ直後”にdump**していくと、1〜2回で犯人が決まります。

### 3-2) タスクが犯人っぽい場合

`xTaskCreate()` 等でタスク開始後に変わるなら、

* そのタスクの `setup` 相当（最初の数行）で `dumpPins("task X started")` を出す
* さらに、タスクが周期処理なら「最初の1回目の処理直後」でもdump

で、**どのタスクがピンを奪うか**が特定できます。

---

## 4) “コードベース検証”を最短化するための結論（ありがちな犯人トップ2）

ここまでの監査をやると、多くの場合は次のどちらかに収束します。

### A) LEDC/analogWrite/tone/Servo がCH1/CH2に残留

* `gpio_dump_io_configuration` の `SigOut ID` が256以外になる
* `ledcReadFreq(18/5)` が0以外っぽい値を返す
* “DigitalOutputにしたはずなのに波形がPWM”

→ この場合は **「どこでLEDCにattachしてるか」**を `rg` と Pin Audit差分で確定できます。
LEDCの正しいAPI（`ledcAttach/ledcDetach`等）は公式ドキュメントの通りです。([Espressif Systems][2])

### B) SPI.begin/SD.begin 等がVSPIデフォルトで走ってCH1=18/CH2=5を奪う

SPI Multiple Buses例で、VSPIデフォルトが **SCLK=18 / SS=5** と書かれています。([Espressif Systems][4])
あなたの症状（18と5だけおかしい）がこのパターンに非常に合致します。

---

## 5) 次にあなたがやること（作業指示まとめ）

1. **PinAudit.h を追加**して、`dumpPins()` を作る
2. `setup()` をフェーズ分割して、各フェーズ直後に `dumpPins()` を入れる
3. 監査ログで、GPIO18/5の

   * `OutputEn` / `OpenDrain`
   * `GPIO Matrix SigOut ID`（256かどうか）([Espressif Systems][1])
     を確認し、**どのタイミングで変わるか**を確定
4. その直前フェーズのコードに対して、`rg` 検索（LEDC系/SPI系）をかけて、呼び出し箇所を潰す

   * LEDC/analogWrite系：([Espressif Systems][2])
   * SPIデフォルトピンの根拠：([Espressif Systems][4])

---



[1]: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html "GPIO & RTC GPIO - ESP32 -  — ESP-IDF Programming Guide v5.5.2 documentation"
[2]: https://docs.espressif.com/projects/arduino-esp32/en/latest/api/ledc.html "LED Control (LEDC) -  -  — Arduino ESP32 latest documentation"
[3]: https://docs.espressif.com/projects/arduino-esp32/en/latest/migration_guides/2.x_to_3.0.html "Migration from 2.x to 3.0 -  -  — Arduino ESP32 latest documentation"
[4]: https://docs.espressif.com/projects/arduino-esp32/en/latest/api/spi.html "SPI -  -  — Arduino ESP32 latest documentation"
