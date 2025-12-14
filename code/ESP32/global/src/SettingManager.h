#pragma once
#include <Arduino.h>
#include <Preferences.h>

/**
 * SettingManager
 *
 * NVS（Preferences）を使った設定管理
 * - 初回起動時にデフォルト値を投入
 * - getString/setString でkey/value保存
 */
class SettingManager {
public:
  /**
   * 初期化（NVSを開き、デフォルト設定を投入）
   * @return 成功時true
   */
  bool begin();

  /**
   * 文字列設定を取得
   * @param key キー名
   * @param defValue 未設定時のデフォルト値
   * @return 設定値
   */
  String getString(const String& key, const String& defValue = "");

  /**
   * 文字列設定を保存
   * @param key キー名
   * @param value 値
   */
  void setString(const String& key, const String& value);

  /**
   * 整数設定を取得
   * @param key キー名
   * @param defValue 未設定時のデフォルト値
   * @return 設定値
   */
  int getInt(const String& key, int defValue = 0);

  /**
   * 整数設定を保存
   * @param key キー名
   * @param value 値
   */
  void setInt(const String& key, int value);

  /**
   * 設定が存在するか確認
   * @param key キー名
   * @return 存在すればtrue
   */
  bool hasKey(const String& key);

  /**
   * 全設定をクリア（工場出荷リセット）
   */
  void clear();

private:
  Preferences prefs_;
  bool initialized_ = false;

  void initDefaults();
};
