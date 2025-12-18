#pragma once
#include <Arduino.h>
#include <SPIFFS.h>

/**
 * SPIFFSManager
 *
 * SPIFFSファイルシステムの初期化と入出力管理
 */
class SPIFFSManager {
public:
  /**
   * SPIFFS初期化
   * @param formatOnFail 失敗時にフォーマットするか
   * @return 成功時true
   */
  bool begin(bool formatOnFail = true);

  /**
   * ファイル読み込み
   * @param path ファイルパス
   * @return ファイル内容（失敗時は空文字）
   */
  String readFile(const String& path);

  /**
   * ファイル書き込み
   * @param path ファイルパス
   * @param content 内容
   * @return 成功時true
   */
  bool writeFile(const String& path, const String& content);

  /**
   * ファイル追記
   * @param path ファイルパス
   * @param content 追記内容
   * @return 成功時true
   */
  bool appendFile(const String& path, const String& content);

  /**
   * ファイル削除
   * @param path ファイルパス
   * @return 成功時true
   */
  bool deleteFile(const String& path);

  /**
   * ファイル存在確認
   * @param path ファイルパス
   * @return 存在すればtrue
   */
  bool exists(const String& path);

  /**
   * 空き容量取得（バイト）
   */
  size_t freeSpace();

  /**
   * 総容量取得（バイト）
   */
  size_t totalSpace();

  /**
   * 使用容量取得（バイト）
   */
  size_t usedSpace();

  /**
   * 初期化済みか
   */
  bool isReady() const { return initialized_; }

private:
  bool initialized_ = false;
};
