# AraneaSDK ESP32 ベストプラクティス

ESP32でaraneaDeviceを実装する際の重要な知見をまとめたドキュメント。

---

## 1. HTTPSタイムアウト設定

### 問題

ESP32のHTTPClientでCloud Functions (HTTPS) に接続する際、デフォルトや短いタイムアウトでは`-11 (HTTPC_ERROR_READ_TIMEOUT)` エラーが発生する。

### 原因

| 処理 | 所要時間 |
|------|---------|
| DNS解決 | 0.5-2秒 |
| TCP接続 | 0.5-1秒 |
| **SSL/TLSハンドシェイク** | **3-10秒** |
| HTTPリクエスト/レスポンス | 0.5-2秒 |

**SSL handshakeがボトルネック**。ESP32のmbedTLSは証明書チェーン検証に時間がかかる。

### 解決策

```cpp
// NG: 3秒では不足
http.setTimeout(3000);

// OK: 15秒以上を推奨
http.setTimeout(15000);
```

### 推奨設定

| エンドポイント | 推奨タイムアウト |
|---------------|-----------------|
| araneaDeviceGate (登録) | 15秒 |
| deviceStateReport | 15秒 |
| CelestialGlobe webhook | 10秒 |
| ローカルRelay (HTTP) | 3秒 |

### HTTPClientエラーコード一覧

```cpp
#define HTTPC_ERROR_CONNECTION_REFUSED  (-1)
#define HTTPC_ERROR_SEND_HEADER_FAILED  (-2)
#define HTTPC_ERROR_SEND_PAYLOAD_FAILED (-3)
#define HTTPC_ERROR_NOT_CONNECTED       (-4)
#define HTTPC_ERROR_CONNECTION_LOST     (-5)
#define HTTPC_ERROR_NO_STREAM           (-6)
#define HTTPC_ERROR_NO_HTTP_SERVER      (-7)
#define HTTPC_ERROR_TOO_LESS_RAM        (-8)   // メモリ不足
#define HTTPC_ERROR_ENCODING            (-9)
#define HTTPC_ERROR_STREAM_WRITE        (-10)
#define HTTPC_ERROR_READ_TIMEOUT        (-11)  // タイムアウト
```

---

## 2. メモリ管理

### SSL接続に必要なメモリ

| 項目 | 必要メモリ |
|------|-----------|
| HTTPClient + SSL | 約40KB |
| ArduinoJson (4096) | 約4KB |
| 文字列バッファ | 約2KB |
| **合計** | **約50KB以上必要** |

### ヒープ監視

```cpp
Serial.printf("Free heap: %d, Largest block: %d\n",
              ESP.getFreeHeap(),
              ESP.getMaxAllocHeap());

// 警告: 50KB以下でHTTPS接続が不安定に
if (ESP.getFreeHeap() < 50000) {
  Serial.println("WARNING: Low memory, HTTPS may fail");
}
```

### メモリ節約テクニック

```cpp
// 1. StaticJsonDocumentを使用（ヒープ断片化防止）
StaticJsonDocument<2048> doc;  // スタック上に確保

// 2. String::reserve()で再アロケーション防止
String json;
json.reserve(1024);
serializeJson(doc, json);

// 3. HTTPClient は使用後すぐに終了
HTTPClient http;
http.begin(url);
int code = http.POST(payload);
String response = http.getString();
http.end();  // ← 忘れずに！
yield();     // WDT対策
```

---

## 3. API エンドポイント

### リージョン別エンドポイント

| API | リージョン | URL |
|-----|-----------|-----|
| araneaDeviceGate | us-central1 | `https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate` |
| deviceStateReport | asia-northeast1 | `https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport` |
| CelestialGlobe | us-central1 | `https://us-central1-mobesorder.cloudfunctions.net/celestialGlobe_ingest` |

**注意**: araneaDeviceGateはasia-northeast1に存在しない！

---

## 4. 再試行とバックオフ

### 推奨パターン

```cpp
static const int MAX_RETRIES = 3;
static const unsigned long BACKOFF_MS = 30000;
static int consecutiveFailures = 0;
static unsigned long lastFailTime = 0;

bool sendWithRetry() {
  // バックオフ中はスキップ
  if (consecutiveFailures >= MAX_RETRIES) {
    if (millis() - lastFailTime < BACKOFF_MS) {
      return false;  // バックオフ中
    }
    consecutiveFailures = 0;  // バックオフ解除
  }

  // 送信処理
  bool success = sendReport();

  if (success) {
    consecutiveFailures = 0;
  } else {
    consecutiveFailures++;
    lastFailTime = millis();
  }

  return success;
}
```

---

## 5. デバッグTips

### シリアルログ出力例

```cpp
Serial.printf("[HTTP] URL: %s\n", url.c_str());
Serial.printf("[HTTP] Payload: %d bytes\n", payload.length());
Serial.printf("[HTTP] Free heap before: %d\n", ESP.getFreeHeap());

int httpCode = http.POST(payload);

Serial.printf("[HTTP] Response code: %d\n", httpCode);
if (httpCode > 0) {
  Serial.printf("[HTTP] Response: %s\n", http.getString().c_str());
} else {
  Serial.printf("[HTTP] Error: %s\n", http.errorToString(httpCode).c_str());
}
Serial.printf("[HTTP] Free heap after: %d\n", ESP.getFreeHeap());
```

### よくある問題と対処

| 症状 | 原因 | 対処 |
|------|------|------|
| -11 READ_TIMEOUT | タイムアウト短い | 15秒以上に |
| -8 TOO_LESS_RAM | メモリ不足 | ヒープ確保、JSON縮小 |
| -1 CONNECTION_REFUSED | サーバー停止/URL間違い | エンドポイント確認 |
| 401 Unauthorized | CIC不一致 | 再登録 |
| 404 Not Found | リージョン間違い | URL確認 |

---

## 6. 実装チェックリスト

- [ ] HTTPタイムアウト 15秒以上
- [ ] ヒープ監視ログ追加
- [ ] StaticJsonDocument使用
- [ ] http.end() 呼び出し
- [ ] yield() でWDT対策
- [ ] バックオフ実装
- [ ] エラーコードログ出力

---

## 変更履歴

| 日付 | 内容 |
|------|------|
| 2025-12-27 | 初版作成（HTTPタイムアウト問題の知見） |
