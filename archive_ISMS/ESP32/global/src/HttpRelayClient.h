#pragma once
#include <Arduino.h>
#include <HTTPClient.h>

/**
 * HTTP送信結果
 */
struct HttpRelayResult {
  bool ok;
  int httpCode;
  String error;
  String url;
};

/**
 * HttpRelayClient
 *
 * is03（Zero3）へのHTTP中継クライアント
 * - primary -> secondary フォールバック
 */
class HttpRelayClient {
public:
  /**
   * コンストラクタ
   * @param primaryUrl primary URL
   * @param secondaryUrl secondary URL（フォールバック）
   */
  HttpRelayClient(const String& primaryUrl, const String& secondaryUrl);

  /**
   * JSON POSTを実行（primary -> secondary フォールバック）
   * @param json JSON文字列
   * @return 結果
   */
  HttpRelayResult postJson(const String& json);

  /**
   * 指定URLにJSON POST
   * @param url URL
   * @param json JSON文字列
   * @return 結果
   */
  HttpRelayResult postJsonTo(const String& url, const String& json);

  /**
   * URLを更新
   * @param primaryUrl primary URL
   * @param secondaryUrl secondary URL
   */
  void setUrls(const String& primaryUrl, const String& secondaryUrl);

  /**
   * primary URL取得
   */
  String getPrimaryUrl() const { return primaryUrl_; }

  /**
   * secondary URL取得
   */
  String getSecondaryUrl() const { return secondaryUrl_; }

private:
  String primaryUrl_;
  String secondaryUrl_;

  static const int TIMEOUT_MS = 5000;
};
