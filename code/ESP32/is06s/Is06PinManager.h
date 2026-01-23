/**
 * Is06PinManager.h
 *
 * IS06S PIN制御マネージャー
 *
 * 【機能】
 * - CH1-4 (D/P Type): Digital Output / PWM Output
 * - CH5-6 (I/O Type): Digital Input / Digital Output
 * - PIN enabled/disabled制御
 * - PINglobal参照チェーン
 * - 状態変化コールバック
 *
 * 【設計原則】
 * - シングルタスク設計（loop()でupdate()呼び出し）
 * - ハードコード排除（AraneaSettingsDefaults参照）
 * - NVS永続化対応
 */

#ifndef IS06_PIN_MANAGER_H
#define IS06_PIN_MANAGER_H

#include <Arduino.h>
#include "SettingManager.h"
#include "AraneaSettingsDefaults.h"

// ============================================================
// 定数定義
// ============================================================
static const int IS06_CHANNEL_COUNT = 6;
static const int IS06_DP_CHANNELS = 4;   // CH1-4 (D/P Type)
static const int IS06_IO_CHANNELS = 2;   // CH5-6 (I/O Type)

// GPIO割り当て
static const int IS06_PIN_MAP[IS06_CHANNEL_COUNT] = {
  18,   // CH1 (D/P)
  5,    // CH2 (D/P)
  15,   // CH3 (D/P)
  27,   // CH4 (D/P)
  14,   // CH5 (I/O)
  12    // CH6 (I/O)
};

// LEDCチャンネル（PWM用）
static const int IS06_LEDC_CHANNEL[IS06_DP_CHANNELS] = {0, 1, 2, 3};

// ============================================================
// 列挙型
// ============================================================
enum class PinType {
  DIGITAL_OUTPUT,
  PWM_OUTPUT,
  DIGITAL_INPUT,
  PIN_DISABLED
};

enum class ActionMode {
  MOMENTARY,    // モーメンタリ（パルス）
  ALTERNATE,    // オルタネート（トグル）
  SLOW,         // PWM: なだらか変化
  RAPID,        // PWM: 即座変化
  ROTATE        // 入力: ローテート
};

// ============================================================
// PIN設定構造体
// ============================================================
struct PinSetting {
  PinType type = PinType::DIGITAL_OUTPUT;
  ActionMode actionMode = ActionMode::MOMENTARY;
  String name = "";
  int validity = 3000;        // モーメンタリ動作時間 (ms)
  int debounce = 3000;        // デバウンス時間 (ms)
  int rateOfChange = 4000;    // PWM変化時間 (ms)
  int expiry = 1;             // 有効期限 (日)
  bool expiryEnabled = false;
  String expiryDate = "";     // 有効期限日時 (YYYYMMDDHHMM)
  String allocation[4] = {"", "", "", ""};  // 入力時の連動先（最大4ch）
  int allocationCount = 0;
  int pwmPresets[4] = {0, 30, 60, 100};  // PWMプリセット値
  int presetCount = 4;
};

// ============================================================
// PIN状態構造体
// ============================================================
struct PinState {
  bool enabled = true;        // PIN有効/無効
  int digitalState = 0;       // Digital: 0=OFF, 1=ON
  int pwmValue = 0;           // PWM: 0-100%
  unsigned long lastAction = 0;
  unsigned long pulseEndTime = 0;  // モーメンタリパルス終了時刻
  unsigned long debounceEnd = 0;   // デバウンス終了時刻
  bool inputState = false;         // 入力状態
  bool lastInputState = false;     // 前回入力状態
  int currentPresetIndex = 0;      // PWMローテート用
};

// ============================================================
// コールバック型
// ============================================================
typedef void (*PinStateCallback)(int channel, int state, int pwmValue);
typedef void (*PinInputCallback)(int channel, bool state);

// ============================================================
// Is06PinManager クラス
// ============================================================
class Is06PinManager {
public:
  /**
   * 初期化
   * @param settings SettingManager参照
   */
  void begin(SettingManager* settings);

  /**
   * 毎ループ更新（パルス終了、PWM遷移、入力検知）
   */
  void update();

  // --- PIN制御 ---
  /**
   * PIN状態を設定
   * @param channel チャンネル番号 (1-6)
   * @param state Digital: 0/1, PWM: 0-100
   * @return 成功時true, enabled=falseやバリデーションエラー時false
   */
  bool setPinState(int channel, int state);

  /**
   * PIN状態を取得
   * @param channel チャンネル番号 (1-6)
   * @return 現在の状態
   */
  int getPinState(int channel);

  /**
   * PWM値を設定
   * @param channel チャンネル番号 (1-4)
   * @param value PWM値 (0-100%)
   * @return 成功時true
   */
  bool setPwmValue(int channel, int value);

  /**
   * PWM値を取得
   * @param channel チャンネル番号 (1-4)
   * @return PWM値 (0-100%)
   */
  int getPwmValue(int channel);

  // --- PIN有効/無効制御 (P1-1a) ---
  /**
   * PINが有効か確認
   * @param channel チャンネル番号 (1-6)
   * @return 有効ならtrue
   */
  bool isPinEnabled(int channel);

  /**
   * PIN有効/無効を設定
   * @param channel チャンネル番号 (1-6)
   * @param enabled 有効/無効
   */
  void setPinEnabled(int channel, bool enabled);

  // --- 設定管理 ---
  /**
   * PIN設定を取得
   * @param channel チャンネル番号 (1-6)
   * @return PinSetting構造体への参照
   */
  const PinSetting& getPinSetting(int channel);

  /**
   * PIN設定を更新
   * @param channel チャンネル番号 (1-6)
   * @param setting 新しい設定
   */
  void setPinSetting(int channel, const PinSetting& setting);

  /**
   * PINタイプを設定
   * @param channel チャンネル番号 (1-6)
   * @param type PINタイプ
   */
  void setPinType(int channel, PinType type);

  // --- PINglobal参照チェーン (P1-1b) ---
  /**
   * 有効なValidity値を取得（PINSettings → PINglobal → デフォルト）
   * @param channel チャンネル番号 (1-6)
   * @return Validity値 (ms)
   */
  int getEffectiveValidity(int channel);

  /**
   * 有効なDebounce値を取得
   * @param channel チャンネル番号 (1-6)
   * @return Debounce値 (ms)
   */
  int getEffectiveDebounce(int channel);

  /**
   * 有効なRateOfChange値を取得
   * @param channel チャンネル番号 (1-4)
   * @return RateOfChange値 (ms)
   */
  int getEffectiveRateOfChange(int channel);

  // --- コールバック ---
  /**
   * 状態変化コールバックを設定
   */
  void setStateCallback(PinStateCallback callback);

  /**
   * 入力検知コールバックを設定
   */
  void setInputCallback(PinInputCallback callback);

  // --- ユーティリティ ---
  /**
   * 全PIN状態をシリアル出力
   */
  void printStatus();

  /**
   * NVSから設定を読み込み
   */
  void loadFromNvs();

  /**
   * NVSに設定を保存
   */
  void saveToNvs();

private:
  SettingManager* settings_ = nullptr;
  PinSetting pinSettings_[IS06_CHANNEL_COUNT];
  PinState pinStates_[IS06_CHANNEL_COUNT];

  PinStateCallback stateCallback_ = nullptr;
  PinInputCallback inputCallback_ = nullptr;

  // PWM遷移管理
  int pwmTargetValue_[IS06_DP_CHANNELS] = {0, 0, 0, 0};
  unsigned long pwmTransitionStart_[IS06_DP_CHANNELS] = {0, 0, 0, 0};
  int pwmStartValue_[IS06_DP_CHANNELS] = {0, 0, 0, 0};

  // 内部メソッド
  void initGpio();
  void initLedc();
  void handleMomentaryPulse();
  void handlePwmTransition();
  void handleDigitalInput();
  void triggerAllocations(int inputChannel);
  void applyDigitalOutput(int channel, int state);
  void applyPwmOutput(int channel, int value);

  // NVSキー生成
  String getNvsKey(int channel, const char* suffix);

  // チャンネル検証
  bool isValidChannel(int channel);
  bool isValidDpChannel(int channel);
  bool isValidIoChannel(int channel);
};

#endif // IS06_PIN_MANAGER_H
