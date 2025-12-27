# is06a - 6ch Relay Controller 仕様書

## 概要

ar-is06a は6チャンネルのリレー/トリガー出力デバイス。
HTTP API経由でパルス出力・PWM制御を行い、状態変化をローカルリレー/クラウドへ通知する。

## チャンネル構成（6CH）

| チャンネル | GPIO | 機能 | 備考 |
|-----------|------|------|------|
| TRG1 | GPIO12 | デジタル出力 + PWM | ストラップピン（※1） |
| TRG2 | GPIO14 | デジタル出力 + PWM | - |
| TRG3 | GPIO27 | デジタル出力 + PWM | - |
| TRG4 | GPIO15 | デジタル出力 + PWM | ストラップピン（※2） |
| TRG5 | GPIO5 | I/O切替（出力 or 入力） | ストラップピン（※3） |
| TRG6 | GPIO18 | I/O切替（出力 or 入力） | - |

### TRG1-4（PWM対応）

- モード: `digital`（デフォルト）または `pwm`
- デジタルモード: パルス出力（10ms〜10000ms）
- PWMモード: 周波数1kHz〜40kHz、デューティ0〜255（8bit）
- NVS保存: モード、パルス幅、PWM周波数、PWMデューティ

### TRG5-6（I/O切替可能）

- モード: `output`（デフォルト）または `input`
- 出力モード: パルス出力（10ms〜10000ms）
- 入力モード: デバウンス付きエッジ検出（立上り/立下り）
- NVS保存: I/Oモード、パルス幅、デバウンス時間

## ストラップピンの注意事項

### ※1 GPIO12 (MTDI) - フラッシュ電圧選択

ESP32のブート時にGPIO12の電圧レベルでフラッシュ電圧を決定する：
- **LOW**: 3.3Vフラッシュ（標準、ほとんどのモジュールはこれ）
- **HIGH**: 1.8Vフラッシュ（一部の特殊なモジュールのみ）

**運用実績**: is04a（2ch接点出力）でGPIO12/14を使用し、問題なく稼働中。

**推奨対策**:
- 内部プルダウン設定（コード側で対応）
- または、外部回路でGPIO12を10kΩプルダウン抵抗でLOWに固定

### ※2 GPIO15 (MTDO) - ブートログ制御

ESP32のブート時にGPIO15の電圧レベルでシリアルログ出力を制御：
- **HIGH**: シリアルブートログ出力（デフォルト）
- **LOW**: シリアルブートログ抑制

**コード側対策**（TriggerManagerIs06a.cpp）:
1. `begin()`でTRG4(GPIO15)は**IOControllerを使わず直接GPIO制御**で初期化
2. `digitalWrite(HIGH)` → `pinMode(OUTPUT)` の順序でLOWパルスを完全回避
3. DIGITALモード時はアイドル状態を**HIGH**に維持
4. `setMode()`でもIOControllerを迂回し、直接GPIO制御を使用

**理由**: IOControllerのtransitionToOutput()はdigitalWrite(pin, LOW)を先に実行するため、
GPIO15を通常の出力ピンとして初期化するとLOWに引っ張られ、不安定化する可能性がある。

**制約**: TRG4はIOControllerを使用しないため、IOController経由の状態取得や共通機能
（デバウンス等）は利用不可。PWM制御はLEDC API、デジタル制御は直接GPIO操作で行う。

**運用根拠**: GPIO12がis04aで問題なく動作しているため、同様の対策でGPIO15も運用可能と判断。

**ハードウェア確認**: 基板側でGPIO12/15に10kΩプル抵抗が実装されているか実機で要確認。

### ※3 GPIO5 - ブートログレベル制御

ESP32のブート時にGPIO5の電圧レベルでシリアルログ出力を制御：
- **HIGH**: シリアルブートログ出力（デフォルト）
- **LOW**: シリアルブートログ抑制

**影響**: 通常の動作には影響なし。デバッグ時のログ表示のみ。

## NVSキー一覧

```
is06_intrlk   : インターロック時間(ms)
is06_t1_nm    : TRG1名称
is06_t1_mode  : TRG1モード ("digital" / "pwm")
is06_t1_pls   : TRG1パルス幅(ms)
is06_t1_freq  : TRG1 PWM周波数(Hz)
is06_t1_duty  : TRG1 PWMデューティ(0-255)
is06_t2_nm    : TRG2名称
is06_t2_mode  : TRG2モード
is06_t2_pls   : TRG2パルス幅(ms)
is06_t2_freq  : TRG2 PWM周波数(Hz)
is06_t2_duty  : TRG2 PWMデューティ(0-255)
is06_t3_nm    : TRG3名称
is06_t3_mode  : TRG3モード
is06_t3_pls   : TRG3パルス幅(ms)
is06_t3_freq  : TRG3 PWM周波数(Hz)
is06_t3_duty  : TRG3 PWMデューティ(0-255)
is06_t4_nm    : TRG4名称
is06_t4_mode  : TRG4モード ("digital" / "pwm")
is06_t4_pls   : TRG4パルス幅(ms)
is06_t4_freq  : TRG4 PWM周波数(Hz)
is06_t4_duty  : TRG4 PWMデューティ(0-255)
is06_t5_nm    : TRG5名称
is06_t5_io    : TRG5 I/Oモード ("output" / "input")
is06_t5_pls   : TRG5パルス幅(ms)
is06_t5_deb   : TRG5デバウンス(ms)
is06_t6_nm    : TRG6名称
is06_t6_io    : TRG6 I/Oモード
is06_t6_pls   : TRG6パルス幅(ms)
is06_t6_deb   : TRG6デバウンス(ms)
is06_hb_sec   : 心拍間隔(秒)
is06_boot_g   : 起動猶予期間(ms)
```

## HTTP API

### パルス実行

```
POST /api/is06a/trigger
Content-Type: application/json

{
  "trigger": 1,       // 1-6
  "duration": 3000    // ms (optional)
}
```

### PWM設定

```
POST /api/is06a/pwm
Content-Type: application/json

{
  "trigger": 1,       // 1-4 only
  "duty": 128         // 0-255
}
// または
{
  "trigger": 1,
  "duty_percent": 50.0  // 0-100%
}
```

### 設定取得/変更

```
GET /api/is06a/config
POST /api/is06a/config
```

## 安全機能

### インターロック

連続パルス防止。前回パルス終了後、設定時間（デフォルト200ms）経過するまで次のパルスを拒否。

### 起動猶予期間

電源投入後、設定時間（デフォルト3000ms）は状態送信をスキップ。電源ノイズによる誤報告を防止。

### 入力デバウンス

TRG5-6が入力モード時、IOControllerのデバウンス処理（デフォルト100ms）でチャタリング除去。
追加で50msのクールダウンを設けてエッジ検出の連続発火を防止。

## LEDC PWM実装詳細

ESP32 Arduino Core 3.x のLEDC APIを使用：

```cpp
// チャンネル取得
int ch = ledcAttach(pin, freq, 8);  // 8bit resolution

// デューティ設定（チャンネル指定版）
ledcWriteChannel(ch, duty);

// 周波数変更（チャンネル指定版）
ledcChangeFrequency(ch, freq, 8);

// PWM解除
ledcDetach(pin);
```

**注意**: `ledcWrite(pin, duty)` は非推奨。必ず `ledcWriteChannel(ch, duty)` を使用。

### LEDC確保失敗時のフォールバック

`ledcAttach()` が失敗した場合（ch < 0）、自動的にデジタルモードに切り替え。
ユーザーはシリアルログで確認可能。

## 設計上の制約と注意

1. **シングルタスク設計**: loop()ベース、セマフォ/WDT不使用
2. **パーティション**: min_spiffs必須（OTA領域確保）
3. **状態送信タイムアウト**: 1.5秒×3エンドポイント = 最大4.5秒
4. **PWM起動時復元**: NVSから読み込んだデューティを起動時に適用
5. **TRG4(GPIO15)特殊制御**: IOController不使用、直接GPIO/LEDC制御のみ（※2参照）

## ビルド情報

- フラッシュ使用量: 約66% (1,305,637バイト)
- RAM使用量: 約16% (54,136バイト)

## 変更履歴

- 2024-12: 5CH→6CH復元（GPIO15追加、内部プル抵抗で対応）
- 2024-12: 6CH→5CH変更（GPIO15ストラップピンリスク排除）
- 2024-12: LEDC API統一（ledcWriteChannel使用）
- 2024-12: PWM起動時復元、LEDC失敗フォールバック追加
