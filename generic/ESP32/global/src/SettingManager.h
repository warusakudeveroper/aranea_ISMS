#pragma once
#include <Arduino.h>
#include <Preferences.h>

/**
 * SettingManager
 *
 * NVS（Preferences）を使った汎用設定管理クラス
 * - 純粋なCRUD操作のみ提供
 * - デバイス固有のデフォルト値設定は各デバイスのAraneaSettingsに委譲
 *
 * 使用例:
 *   SettingManager settings;
 *   settings.begin("myns");                    // 名前空間を指定して初期化
 *   AraneaSettings::initDefaults(settings);   // デバイス固有のデフォルト設定
 *   String val = settings.getString("key");
 */
class SettingManager {
public:
  /**
   * 初期化（NVSを開く）
   * @param ns NVS名前空間（デフォルト: "isms"）
   * @return 成功時true
   */
  bool begin(const char* ns = "isms");

  /**
   * 終了（NVSを閉じる）
   */
  void end();

  /**
   * 初期化済みか確認
   */
  bool isInitialized() const { return initialized_; }

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
   * unsigned long設定を取得
   * @param key キー名
   * @param defValue 未設定時のデフォルト値
   * @return 設定値
   */
  unsigned long getULong(const String& key, unsigned long defValue = 0);

  /**
   * unsigned long設定を保存
   * @param key キー名
   * @param value 値
   */
  void setULong(const String& key, unsigned long value);

  /**
   * bool設定を取得
   * @param key キー名
   * @param defValue 未設定時のデフォルト値
   * @return 設定値
   */
  bool getBool(const String& key, bool defValue = false);

  /**
   * bool設定を保存
   * @param key キー名
   * @param value 値
   */
  void setBool(const String& key, bool value);

  /**
   * 設定が存在するか確認
   * @param key キー名
   * @return 存在すればtrue
   */
  bool hasKey(const String& key);

  /**
   * 特定のキーを削除
   * @param key キー名
   * @return 削除成功時true
   */
  bool remove(const String& key);

  /**
   * 全設定をクリア（この名前空間のみ）
   */
  void clear();

  /**
   * 現在の名前空間を取得
   */
  const char* getNamespace() const { return namespace_; }

private:
  Preferences prefs_;
  bool initialized_ = false;
  char namespace_[16] = "isms";
};
