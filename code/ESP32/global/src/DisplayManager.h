#pragma once
#include <Arduino.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>

/**
 * DisplayManager
 *
 * 0.96" OLED SSD1306 128x64 I2C表示管理
 */
class DisplayManager {
public:
  /**
   * 初期化（I2C初期化、ディスプレイ起動）
   * @return 成功時true
   */
  bool begin();

  /**
   * ブート画面表示
   * @param msg メッセージ
   */
  void showBoot(const String& msg);

  /**
   * エラー画面表示
   * @param msg エラーメッセージ
   */
  void showError(const String& msg);

  /**
   * クリア
   */
  void clear();

  /**
   * 4行表示（is02用）
   * @param line1 1行目（IP等）
   * @param line2 2行目（自機ID等）
   * @param line3 3行目（センサー情報等）
   * @param line4 4行目（状態等）
   */
  void show4Lines(const String& line1, const String& line2,
                  const String& line3, const String& line4);

  /**
   * 特定行を更新
   * @param lineNum 行番号（0-3）
   * @param text テキスト
   */
  void updateLine(int lineNum, const String& text);

  /**
   * 表示を更新（バッファをディスプレイに反映）
   */
  void display();

  /**
   * is02メイン画面表示
   * @param line1 1行目（IP or 接続状態）- 小さいフォント
   * @param cic CICコード（大きいフォントで2行使用）
   * @param sensorLine センサー情報（温度/湿度 or エラー）
   * @param showLink 送信中リンクマーク表示
   */
  void showIs02Main(const String& line1, const String& cic,
                    const String& sensorLine, bool showLink = false);

  /**
   * 接続中ティッカー表示
   * @param ssid 接続試行中のSSID
   * @param frame アニメーションフレーム（0-3）
   */
  void showConnecting(const String& ssid, int frame);

  /**
   * 登録処理中表示
   * @param status ステータスメッセージ
   */
  void showRegistering(const String& status);

  /**
   * is01メイン画面表示（電池式センサー用）
   * @param cic CICコード（大きいフォント）
   * @param sensorLine センサー情報（温度/湿度）
   * @param infoLine 追加情報（ブート回数等）
   */
  void showIs01Main(const String& cic, const String& sensorLine,
                    const String& infoLine);

private:
  Adafruit_SSD1306* oled_ = nullptr;
  bool initialized_ = false;

  static const int SCREEN_WIDTH = 128;
  static const int SCREEN_HEIGHT = 64;
  static const int OLED_ADDR = 0x3C;
  static const int LINE_HEIGHT = 16;
};
