# レビュー1

• 解析結果（review11に基づく）

  - CH1/CH2初期化でgpio_reset_pinが一度も呼ばれない経路が残存。initLedcとapplyLoadedPinTypesのDigital出力パスは
    gpio_pad_select_gpio/gpio_matrix_outのみで、VSPI等が握っていても強制解除しません（code/ESP32/is06s/
    Is06PinManager.cpp:53-92, 97-170）。setPinTypeでのみgpio_reset_pinを実行していますが、タイプ変更が発生しない限り
    起動時に実行されません（code/ESP32/is06s/Is06PinManager.cpp:520-559）。
  - 出力時もapplyDigitalOutputはgpio_reset_pinを行わず、LEDCデタッチ＋IO_MUX再設定だけです（code/ESP32/is06s/
    Is06PinManager.cpp:1259-1297）。review11で提示された「周辺機能の信号停止→再設定」の決定版対策（reset→pad
    select→matrix→config）が未適用。
  - gpiotestの最小コードは毎回gpio_reset_pin→gpio_set_direction→gpio_set_levelで駆動しており、本実装とは初期化手順が
    異なります（code/ESP32/is06s/gpiotest/gpiotest.ino:14-26）。この差分が「1.3V中間電位」回避の鍵になりうるが、本体
    コードには取り込まれていません。
  - /api/debug/gpio診断はGPIO Matrix/レジスタを読むだけで、ハードが高速トグル/衝突してもsigOutId=256のまま見える可能
    性があります（code/ESP32/is06s/HttpManagerIs06s.cpp:1108-1184）。review11が指摘する「平均1.3V」症状はこの診断で
    は検出できないままです。
  - UT計画のGPIO出力確認（例: UT-GPIO-01/02 in code/ESP32/is06s/design/IS06S_TestPlan.md）の実行記録がなく、現状の
    ファームで「3.3Vを実際に出せている」検証が未完とみなせます。

  結論
  現行実装はreview11で推奨された「周辺機能の強制停止（gpio_reset_pin）」がCH1/CH2の通常パスに組み込まれておらず、
  VSPI等の残留ドライブや高周波トグルを抑止できていません。診断APIもこの症状を検知できない構造です。安全・安定保証は
  現時点で行えません。

  推奨アクション

  1. CH1-4の初期化・出力パスにgpio_reset_pinを組み込み（initLedc/applyLoadedPinTypes/applyDigitalOutput前段）

  ---

  # レビュー2　
  a5b1839 の調査依頼書、かなり「証拠で殴れる」形になってて良いです。特にここが決定打です：

* `sigOutId=256 (Simple GPIO)` で **ペリフェラル上書きなし**
* 高頻度サンプリングで **highCount=1000 / transitions=0（トグル無し）**
* `gpioOutBit=1, gpioEnableBit=1, gpioInBit=1, iomuxFunSel=2, isOpenDrain=false` で **論理的にはSTABLE_HIGH**
  （調査依頼書 3-4 / 3-5 に明記済み）

---

## 重要：あなたが貼った「改訂版報告書」の“SPI発振断定”は、今の実測証拠と矛盾します

改訂版では「1.3V＝発振 or SPIが勝手に出ている可能性が極めて高い」と断定していますが、**それを否定する観測が既に揃っています**：

* **トグル無し（transitions=0）** → “平均電圧として1.3V”のPWM/クロック説は成立しにくい
* **sigOutId=256** → “IO_MUXがSPIに接続されている”説明とも矛盾
* さらに、最小テスト（`gpiotest.ino`）でも **gpio_reset_pin→direction→set_level(1)** を毎回やってなお GPIO18/5だけ再現してる
  → これは「アプリ実装の干渉」より下の層（＝**外部負荷/配線/測定点/ボード回路**)を強く示します

なので、改訂版の「SPIが極めて高い」「gpio_reset_pinが唯一」みたいな言い切りは、**調査書の信頼性を落とすので削るのが正解**です。

---

## 「1.25〜1.37V」の意味（“発振以外”で普通に起きる）

1.3V前後は **“何かがクランプしている/過負荷でドロップしている/測定点がGPIOパッド直じゃない”** でも普通に出ます。

特に、調査書の「3.3Vピン直結で外部が動く → 外部問題ではない」は、**論理的には排除になりません**。
3.3Vレギュレータ出力はGPIOより余裕のある供給源なので、外部入力が想定以上に電流を食っていた場合に「3.3V直結だと動くけどGPIOだと電圧が落ちる」は成立します。

---

## 設置制約のまま、コードだけでさらに“外部起因”を確定させるテスト（おすすめ）

いまの `gpiotest.ino` は「出力Highにできるか」だけ見ています。次は **“その1.3Vが外部バイアスなのか、出力ドロップなのか”** を切ります。

### 追加テスト案（30秒刻みで状態を固定して、電圧を見る）

各ピンで順番にこれをやる：

1. **INPUT + FLOAT**（外部が勝手に1.3Vを作ってるか）
2. **INPUT + PULLUP**（引き上げられるか）
3. **OUTPUT LOW**（0Vまで落ちるか）
4. **OUTPUT HIGH + DRIVE_CAP_MAX**（ドライブ強化で上がるか）

※シリアル無しでも、各状態を30秒〜60秒固定すればテスターで追えます。

ポイント：

* **(1)で既に1.3V** → 外部がバイアス/クランプしてる可能性が濃厚
* **(3)でも0Vに落ちない** → 外部が強く引っ張ってる/測定点がGPIOパッド直じゃない
* **(4)で電圧が上がる** → “外部が電流を吸っていてGPIOが負けてる”を示唆

---

## 調査報告（改訂版）の“差し替え推奨”文章（コピペ用）

> **現象:** GPIO18/GPIO5 を出力HIGHに設定しても実測が 1.25〜1.37V で頭打ち。
> **観測:** GPIO Matrix out_sel は `256 (SIG_GPIO_OUT_IDX / Simple GPIO)`、高頻度サンプリングは `transitions=0` でトグル無し。`GPIO_OUT/ENABLE/IN` はいずれも 1、IOMUX は GPIO function、OpenDrain ではない。
> **解釈:** 周辺ペリフェラル（SPI/LEDC等）による上書きやPWM/クロック発振の可能性は低い。
> **示唆:** “論理的にはHIGHだが物理電圧が上がらない”は、(a) 外部回路によるクランプ/過負荷、(b) 測定点がGPIOパッド直ではない（直列素子/グランド差分）、(c) ボード配線/回路要因、を優先して疑うべき。
> **次の検証:** INPUT(FLOAT/PULLUP) と OUTPUT(LOW/HIGH+最大ドライブ) の状態固定テストで、外部起因か出力ドロップかを確定する。

---

# レビュー3
ご指摘ありがとうございます。「出力がない（0V）」ではなく、**「中途半端な電圧（1.25V〜1.37V）が出ている」** という事実は、トラブルシューティングの方向性を決定的に変える極めて重要な情報です。

この電圧値は、デジタル回路における「不定（Floating）」や「完全なLow（0V）」ではありません。**「何らかの信号が高速で動いている（発振）」** か、**「ドライバ同士が衝突している」** ことを示唆します。

前提を再整理し、この「1.3V前後」という現象に即した調査報告に修正します。

---

# 調査報告書（改訂版）：GPIO18/GPIO5 中間電位出力問題

**件名:** GPIO18/GPIO5における期待値(3.3V)に対し、実測値が中間電位(约1.3V)となる現象の原因特定
**対象:** Aranea_04 / ESP32-WROOM-32
**ステータス:** ハードウェア正常 / **出力信号の衝突または意図しない発振** の疑い

## 1. 現象の再定義

* **期待値:** 3.3V (完全なDC High)
* **実測値:** 1.25V 〜 1.37V
* **示唆:** テスター（DCレンジ）がこの電圧を示す場合、ピンは以下のいずれかの状態にある可能性が高い。
1. **高速スイッチング中:** 高周波のクロック信号（PWMやSPI CLK）が出力されており、その実効値（平均電圧）を計測している。
* 例: 3.3V の 40% Duty ≒ 1.32V


2. **出力衝突（Bus Contention）:** 内部で「Low」を出そうとする力と、外部または別の機能が「High」を出そうとする力が拮抗している。



## 2. 原因分析：なぜ「1.3V」なのか

GPIO18 と GPIO5 の役割（VSPI）と電圧値を照らし合わせると、原因は**「意図しないSPIクロックの出力（発振）」**である可能性が極めて高いと断定できる。

### A. GPIO18 (VSPI_CLK) = 1.3V の正体

GPIO18 はハードウェアSPIのクロックピンである。
もしシステム起動時にSPIコントローラーがActiveになった場合、アイドル時またはデータ送信待機時にクロックラインが特定の周波数で駆動、あるいはプルアップ/ダウン抵抗との兼ね合いで中間電位に見える挙動をすることがある。
特に、**3.3Vの矩形波（クロック信号）が出力されている場合、テスターは平均電圧として 1.65V 近辺（Duty比により変動）を表示する。** 1.25V〜1.37V という値は、負荷やDuty比を考慮すると「クロック信号」として非常に合点がいく値である。

### B. GPIO5 (VSPI_SS) = 1.3V の正体

通常、CSピンはHighかLowに固定されるが、もしこれが「他方のSPIデバイス（スレーブ）からの入力」として設定されていたり、あるいはGPIO18と連動して高速にトグルするモードに入っている場合、同様に中間電位が観測される。

### C. 結論：「出力していない」のではなく「別の信号を出している」

「過去のコードでは動いた」→ 当時はこの「裏で動く信号」が止まっていた。
「現在は動かない（1.3V）」→ 現在のビルド環境またはコード構成では、**ユーザーが意識しないところでSPIバスがアクティブになり、クロック等の信号を垂れ流している**。

これが、レジスタ（GPIO_OUT）で「High(1)」を指示しても、物理ピン（IO_MUX）がSPIコントローラーに接続され、勝手に波形を出力している原因である。

## 3. 対策の正当性確認

現象認識は変わったが、解決策の方向性は以前の提案（IO_MUXのリセット）で合致する。ただし、その目的は「Lowからの復帰」ではなく、**「暴走している周辺機能（SPI）の信号停止」** にある。

### 修正された対策コード

この電圧が出ている以上、単なる `pinMode` では上書きできない（周辺機能がドライブ能力を持っている）ため、やはり **`gpio_reset_pin` による強制停止** が必須である。

```cpp
#include <Arduino.h>
#include "driver/gpio.h"

void setup() {
    // 診断用：もしオシロスコープがあれば、この時点でGPIO18に
    // クロック波形が出ている可能性が高い。
    
    // 1. 周辺機能（SPI）による信号出力を強制停止
    //    これにより IO_MUX が GPIOモードに戻り、1.3Vの発振/衝突が止まる
    gpio_reset_pin(GPIO_NUM_18);
    gpio_reset_pin(GPIO_NUM_5);

    // 2. 改めてGPIO出力として設定
    pinMode(18, OUTPUT);
    pinMode(5, OUTPUT);
    
    // 3. 明確なHighを出力
    digitalWrite(18, HIGH);
    digitalWrite(5, HIGH);
}

void loop() {
    // 点滅処理
}

```

## 4. レビューまとめ

* **前提の修正:** 0V（出力なし）ではなく、1.3V（信号出力中/衝突）であった。
* **技術的解釈:** 1.3Vという値は、GPIO18が **SPI Clockとして動作している（発振している）** ことの強い証左である。
* **判定:** コード上の記述に関わらず、コンパイルされたバイナリは「SPIを使用する」構成になっている。これをユーザーコード側でキャンセルするには `gpio_reset_pin()` が唯一の確実な手段である。

# レビュー4

共有いただいた **review10 の検証結果（SigOut ID=256 / トグル無し / レジスタ正常）** を前提にすると、いま貼ってくれた「改訂版」の中に **論理的に成立しない（＝自分の証拠と矛盾する）断定**が混ざっています。
ここを直すだけで、調査依頼書としての説得力が一段上がります。

---

## まず結論：今回の「SPI発振を断定」は、review10 の結果と矛盾します

あなたが示した検証サマリでは、

* **GPIO Matrix SigOut ID = 256（simple GPIO output）**
* 高頻度サンプリングでトグル無し
* レジスタ（OUT/ENABLE/IN）も正常

となっています。

ESP-IDF の GPIO ドキュメントでも、`gpio_dump_io_configuration()` の例として **GPIO18 が “GPIO Matrix SigOut ID: 256 (simple GPIO output)”** のときは、まさに「単純GPIO出力」になっている例が出ています。 ([Espressif Systems][1])
さらに「GPIO matrix 経由で周辺信号が刺さっている場合、ダンプに出る signal ID は gpio_sig_map.h で定義されたものが表示される」と明記されています。 ([Espressif Systems][1])

つまり、**“SPIがIO_MUXを奪って勝手にクロックを出している”** が本当に起きているなら、SigOut ID が 256 のまま、というのは説明しづらいです（SPI信号に対応する別IDになるのが自然）。 ([Espressif Systems][1])

---

## 改訂版で直したほうが良い点（具体）

あなたの改訂版テキストのうち、次は「断定」せず、**仮説扱い**に落とすのが安全です。

### 1) 「1.3V＝発振 or 衝突」の“断定”

DMMで 1.25〜1.37V が見えるのは「平均値」や「押し合い」を示唆しますが、**それだけで発振を確定はできません**。
同じ 1.3V でも、例えば

* ピンが実は Hi-Z（入力/OD等）に近く、外部回路のバイアス点を見ている
* クランプ（ESDダイオード等）で中間電位に固定されている
* 測定点が「GPIOパッド直」ではなく、保護抵抗・トランジスタ段・LED等の“別ノード”で分圧になっている

でも普通に出ます。

今回あなたは **“トグル無し / SigOut 256” を取れている**ので、「高速スイッチング説」はむしろ弱くなっています。

### 2) 「原因は意図しないSPIクロック出力の可能性が極めて高いと断定できる」

これは review10 の結果と衝突します（SigOut 256 が説明できない）。 ([Espressif Systems][1])
SPI関連は **“GPIO18/5 が SPI の典型ピンであるため疑ったが、現時点のレジスタ/マトリクス検証では干渉を示す証拠は出ていない”** が妥当です。

※ちなみに「ArduinoのSPI APIは（典型的には）VSPI側を使う」という説明は公式docにもあります。 ([Espressif Systems][2])
また、コミュニティ資料では VSPI のデフォルト例として SCLK=18 / SS=5 がしばしば挙げられます（ただし“ボード定義依存”のため断定はNG）。 ([ESP32 Forum][3])

### 3) 「gpio_reset_pin() が唯一確実」

あなたの「試行済み修正」に既に `gpio_reset_pin()` が入っていて効果無し、と書いているので、この言い方は避けた方が良いです。
（依頼先に「それもうやったよね？」となる）

---

## “証拠と整合する” 改訂版の差し替え案（そのまま貼れる形）

以下、あなたの文章の骨格を残しつつ、review10 の実績と矛盾しない形に直した案です。

```md
# 調査報告書（改訂案）：GPIO18/GPIO5 中間電位（約1.25〜1.37V）問題

## 1. 現象の再定義
- 期待値: 3.3V（DC High）
- 実測: 約 1.25V〜1.37V（中間電位で安定）
- 補足: DMMのDC測定で中間電位が観測される要因は複数あり、単純な「0V（出力なし）」とは別の状態を示唆する。

## 2. ソフトウェア/ピン制御側の検証結果（review10）
以下の検証により、少なくとも「周辺機能（LEDC/SPI等）がGPIOマトリクス経由でピンを占有し続けている」ことを示す証拠は現時点で確認できていない。
- GPIO Matrix SigOut ID: 256（Simple GPIO Output）= GPIOとして出力経路が成立
- 高頻度サンプリング: トグル無し
- レジスタ直接読み取り: OUT=1 / ENABLE=1 / IN=1 で整合
- gpio_reset_pin / gpio_pad_select_gpio / gpio_matrix_out / gpio_config などによる初期化上書き: 効果なし

→ したがって、現象は「GPIOの論理状態はHighだが、物理ノード電圧が3.3Vまで上がらない（約1.3V付近で頭打ち）」という不一致として整理される。

## 3. 有力仮説（優先度順）
A) 外部回路によるクランプ/分圧/負荷（ピンがHigh駆動しても電圧が引き上げられない）
B) 出力段の駆動能力不足/設定（Drive strength / Open-drain相当の状態）
C) 測定点がGPIOパッド直ではなく、保護抵抗や他素子を挟んだノードで中間電位を見ている
D) 周辺機能の介入（SPI/LEDC等）
※DはGPIO Matrix SigOut ID=256等の検証結果と整合しにくいため、現時点では優先度を下げる。

## 4. 追加で必要な検証（配線変更無しで可能な範囲）
- 入力+PullUp / PullDown / Floating切替時の実測電圧
- 出力High時のDrive strength 切替（CAP0→CAP3）に対する実測電圧の変化
- 出力Low時の実測電圧（0Vまで落ちるか）

## 5. 調査依頼事項
- GPIO18/GPIO5 特有の制約（GPIO5はstrapping pin等）と今回症状への影響可能性の整理
- Arduino-ESP32 3.x 系で「レジスタ整合しているが物理電圧が上がらない」類似事例の有無
- 過去動作コードとの差分観点（初期化順序/ライブラリ/周辺機能利用）で疑うべき点の提示
```

この形にすると、**「1.3V＝SPI発振だ！」**を言い切らず、かつ review10 の成果（干渉否定）をちゃんと活かせます。

---

## “調査依頼書”として追加すると強くなる項目（コードベースで完結）

配線変更できない制約があるので、依頼事項に「実機でこれだけやってほしい（＝コードでできる）」を明示すると強いです。

### 追加依頼：Drive strength / Pull の結果表を埋める

ESP-IDF のGPIOダンプ例にも DriveCap の概念が出てきます。 ([Espressif Systems][1])
GPIO5 は strapping pin であることも明記されています。 ([Espressif Systems][1])

この2点を踏まえ、次の表を埋める指示を依頼書に入れるのが効果的です（DMMで測れる）。

* IN_FLOAT のときの電圧
* IN_PULLUP のときの電圧
* IN_PULLDOWN のときの電圧
* OUT_LOW のときの電圧
* OUT_HIGH（DRIVE_CAP_0/1/2/3）のときの電圧

これで、

* **外部が引っ張ってるのか**
* **ドライブ強化で押し勝てるのか**
  が配線変更無しで判定できます。

---

## 参考：貼っている2つのGitHub Issueの位置づけ

依頼書に入れるなら、下のように整理するのが無難です。

* #11343 は、Arduino-ESP32 3.1以降で **WiFi初期化時に “gpio… reserved” 等のエラーが出る**という話で、例では GPIO2 が出ています。今回の「GPIO18/5が 1.3Vになる」ことを直接説明する内容ではなく、しかも “再現できず” でクローズされています。 ([GitHub][4])
* #9154 は、`pinMode` 周りで **GPIO config failed / ランダムクラッシュ**の報告で、これも「特定の2ピンが中間電位」系の直接根拠にはなりにくいです。 ([GitHub][5])

→ なので「既知問題の可能性として参考リンク」程度にして、**本筋は “SigOut=256で干渉なしなのに電圧が上がらない”**に寄せた方が通ります。

---

## 依頼書に載せる参考リンク（コピペ用）

※URLはそのまま貼れるようにコードブロックに入れます。

```text
ESP-IDF GPIO ドキュメント（SigOut ID 256 の例を含む）
https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html

Arduino-ESP32 SPI ドキュメント（Arduino流だとVSPIを使う等）
https://docs.espressif.com/projects/arduino-esp32/en/latest/api/spi.html

Issue #11343（WiFi init / phy_comm: gpio... reserved など。クローズ）
https://github.com/espressif/arduino-esp32/issues/11343

Issue #9154（pinMode周りのGPIO config failed/クラッシュ系）
https://github.com/espressif/arduino-esp32/issues/9154
```

---

必要なら、あなたが今書いた「改訂版」文章に対して **差分パッチ（どこをどう書き換えるか）**の形で、さらにピンポイントに整形案も出せます。
ただ、現時点で一番重要なのは **“SPI発振の断定”を撤回して、review10 の証拠（SigOut=256/トグル無し）と整合するストーリーに寄せる**ことです。

[1]: https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/peripherals/gpio.html "GPIO & RTC GPIO - ESP32 -  — ESP-IDF Programming Guide v5.5.2 documentation"
[2]: https://docs.espressif.com/projects/arduino-esp32/en/latest/api/spi.html "SPI -  -  — Arduino ESP32 latest documentation"
[3]: https://esp32.com/viewtopic.php?t=40898&utm_source=chatgpt.com "SPI sck synchronization"
[4]: https://github.com/espressif/arduino-esp32/issues/11343 "PHY and GPIO error after 3.1 · Issue #11343 · espressif/arduino-esp32 · GitHub"
[5]: https://github.com/espressif/arduino-esp32/issues/9154 "Gpio pinMode error · Issue #9154 · espressif/arduino-esp32 · GitHub"

---

# レビュー5 (レビュー統合サマリー)

**対象レビュー**: review1〜review4 / 現行実装 (is06s.ino, Is06PinManager.cpp, StateReporterIs06s.cpp, HttpManagerIs06s.cpp, gpiotest.ino)  
**観点**: TLS/StateReporter起因のリブート、PWM/LEDC初期化、GPIO18/5中間電位の両問題をフラットに統合

## 1) TLS/StateReporter 起因リブート疑惑
- review1〜4で疑われた「起動直後のHTTPS送信によるクラッシュ」は、現行コードでは`StateReporterIs06s::begin()`で初回送信を60秒遅延しつつ`update()`は有効のまま（code/ESP32/is06s/is06s.ino, StateReporterIs06s.cpp）。クラウドURLが設定されればHTTPS送信は継続し、根治策（HTTP中継/恒久的setInsecure/リトライ設計見直し）のタスク化が未確認。
- MQTT無効化や一時的コメントアウトによる「止血」フェーズは既に解除されているため、TLS経路が再び実行中。再発有無の実測ログ・UTが欠落しており、安定性の保証は保留。

## 2) PWM/LEDC 初期化とGPIO18/5中間電位
- initLedc/applyLoadedPinTypes/applyDigitalOutputは`gpio_reset_pin`を呼ばずにIO_MUX/Matrixを再設定する設計のまま（code/ESP32/is06s/Is06PinManager.cpp）。CH1/2は起動時に一度も`gpio_reset_pin`を強制されない経路が残存。
- gpiotest最小コードは毎回`gpio_reset_pin`→`gpio_set_direction`→`gpio_set_level`で駆動しており、本体実装との初期化手順差が中間電位再現の切り分けに有効。現行ファームには未反映。
- `/api/debug/gpio`診断はMatrix/レジスタ読取中心で、高速トグルや外部クランプ起因の中間電位は検知困難（sigOutId=256でも発振/外部負荷はあり得る）。

## 3) 実装とレビュー指摘の整合
- review1/2で指摘された「起動即HTTPS送信」「LEDC attach全ch」の切り分けは、一部（初回遅延・ledcDetach後の再設定）は実装済みだが、TLS根治/LEDC安全化の完了宣言なし。
- review3/4で「StateReporter停止ビルドによる安定確認＋恒久策タスク化」を求めたが、現行コードは再有効化済みのため再評価が必要。
- GPIO18/5問題はreview10の証拠（sigOutId=256、トグル無し）とreview11の「SPI発振断定」が衝突したまま。原因未確定として整理し直す必要あり。

## 4) 追加で必要な検証・タスク（提案）
1. TLS経路の再評価: クラウドURL有効状態での連続稼働ログ取得（heap, reset reason, StateReporterログ）と、HTTP中継/完全setInsecure/送信間隔制御の恒久策決定。
2. GPIO18/5: 起動時に`gpio_reset_pin`→`gpio_pad_select_gpio`→`gpio_matrix_out`→`gpio_config`をCH1-4へ組み込み、gpiotest同等の状態固定テスト（INPUT float/pullup/pulldown, OUTPUT low/high+drive強度）をUTとして実測記録。
3. 診断強化: `/api/debug/gpio`にSigOut≠256検知時の即時リセット再設定、ならびに外部クランプ推定のためのdrive強度/入力モード試験結果を返す項目を追加。
4. ドキュメント整合: review11の「SPI発振断定」表現を修正し、review10の実測と矛盾しない形で依頼書を更新。レビュー完了条件を再定義（TLS安定確認とGPIO18/5 3.3V実測を必須UTとして明記）。
