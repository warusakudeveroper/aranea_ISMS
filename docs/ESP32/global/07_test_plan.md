# Test Plan

## 概要

is02のBLEスキャン→HTTP中継動作を確認するためのテスト手順。
is03が未実装でもローカルPCで簡易HTTPサーバを立てて動作確認できる。

## 1. Arduino CLI セットアップ

### 1.1 必要ソフトウェア

```bash
# Arduino CLI インストール（macOS）
brew install arduino-cli

# ESP32 コアインストール
arduino-cli core update-index
arduino-cli core install esp32:esp32

# 必要ライブラリインストール
arduino-cli lib install "NimBLE-Arduino"
arduino-cli lib install "ArduinoJson"
arduino-cli lib install "Adafruit SSD1306"
arduino-cli lib install "Adafruit GFX Library"
```

### 1.2 ボード確認

```bash
# 接続されたESP32を確認
arduino-cli board list
```

## 2. コンパイル

### 2.1 is02 コンパイルコマンド

```bash
cd /path/to/aranea_ISMS

# globalライブラリを参照してコンパイル
arduino-cli compile \
  --fqbn esp32:esp32:esp32 \
  --libraries code/ESP32/global \
  code/ESP32/is02
```

### 2.2 コンパイル成功条件

- エラーなしで完了
- 警告は許容（ただし致命的でないこと）
- 出力例:
  ```
  Sketch uses 1234567 bytes (XX%) of program storage space.
  Global variables use 123456 bytes (XX%) of dynamic memory.
  ```

## 3. 書き込み

```bash
# ポートを指定して書き込み
arduino-cli upload \
  --fqbn esp32:esp32:esp32 \
  --port /dev/cu.usbserial-0001 \
  code/ESP32/is02
```

## 4. シリアルモニタ

```bash
# 115200bps でモニタ
arduino-cli monitor --port /dev/cu.usbserial-0001 --config baudrate=115200
```

## 5. 簡易HTTPサーバ（is03未実装時の代替）

### 5.1 Python版（推奨）

ファイル: `test_server.py`

```python
#!/usr/bin/env python3
"""
is02テスト用 簡易HTTPサーバ
受信したJSONを表示し、200 OKを返す
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class IngestHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)

        print("\n" + "="*60)
        print(f"[{self.path}] Received POST")
        print("="*60)

        try:
            data = json.loads(body.decode('utf-8'))
            print(json.dumps(data, indent=2, ensure_ascii=False))
        except:
            print(f"Raw: {body}")

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{"ok":true}')

    def log_message(self, format, *args):
        pass  # 標準ログを抑制

if __name__ == '__main__':
    port = 8080
    server = HTTPServer(('0.0.0.0', port), IngestHandler)
    print(f"Test server running on http://0.0.0.0:{port}")
    print("Waiting for is02 POST to /api/ingest ...")
    server.serve_forever()
```

実行:
```bash
python3 test_server.py
```

### 5.2 Node.js版

ファイル: `test_server.js`

```javascript
const http = require('http');

const server = http.createServer((req, res) => {
  if (req.method === 'POST') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      console.log('\n' + '='.repeat(60));
      console.log(`[${req.url}] Received POST`);
      console.log('='.repeat(60));
      try {
        console.log(JSON.stringify(JSON.parse(body), null, 2));
      } catch (e) {
        console.log('Raw:', body);
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end('{"ok":true}');
    });
  } else {
    res.writeHead(404);
    res.end();
  }
});

const port = 8080;
server.listen(port, '0.0.0.0', () => {
  console.log(`Test server running on http://0.0.0.0:${port}`);
  console.log('Waiting for is02 POST to /api/ingest ...');
});
```

実行:
```bash
node test_server.js
```

## 6. テスト手順

### 6.1 準備

1. テストPCのIPアドレスを確認
   ```bash
   # macOS/Linux
   ifconfig | grep "inet "
   # 例: 192.168.96.50
   ```

2. is02の `relay.primary` をテストPCに変更
   - 方法A: コード内のデフォルト値を一時的に変更
   - 方法B: NVSに保存済みなら Serial から変更コマンド（未実装なら方法A）

3. 簡易HTTPサーバを起動
   ```bash
   python3 test_server.py
   ```

### 6.2 is02起動確認

シリアルモニタで以下を確認:

```
[BOOT] is02 starting...
[SETTING] Initialized with defaults
[DISPLAY] OLED ready
[WIFI] Trying cluster1...
[WIFI] Connected! IP: 192.168.96.100
[NTP] Time synced: 2025-01-15T10:30:00Z
[LACIS] My lacisId: 3002AABBCCDDEEFF0096
[BLE] Scan started
```

### 6.3 BLE受信確認

is01が近くにある場合（または別のESP32でis01をエミュレート）:

```
[BLE] ISMSv1 detected: staMac=112233445566
[BLE] Sensor lacisId: 3001112233445566770096
[BLE] temp=25.34C, hum=65.20%, batt=80%
[HTTP] POST to http://192.168.96.50:8080/api/ingest
[HTTP] Response: 200 OK
```

### 6.4 テストサーバ側の出力確認

```
============================================================
[/api/ingest] Received POST
============================================================
{
  "gateway": {
    "lacisId": "3002AABBCCDDEEFF0096",
    "ip": "192.168.96.100",
    "rssi": -55
  },
  "sensor": {
    "lacisId": "3001112233445566770096",
    "mac": "112233445566",
    "productType": "001",
    "productCode": "0096"
  },
  "observedAt": "2025-01-15T10:30:00Z",
  "state": {
    "temperatureC": 25.34,
    "humidityPct": 65.2,
    "batteryPct": 80
  },
  "raw": {
    "mfgHex": "4953010107112233445566..."
  }
}
```

## 7. is01エミュレータ（オプション）

is01実機がない場合、別のESP32でBLEアドバタイズを発信してテストする。

ファイル: `is01_emulator.ino`

```cpp
#include <NimBLEDevice.h>

// ISMSv1 ペイロード生成
void setup() {
  Serial.begin(115200);
  NimBLEDevice::init("is01-emu");

  NimBLEServer* pServer = NimBLEDevice::createServer();
  NimBLEAdvertising* pAdv = NimBLEDevice::getAdvertising();

  // Manufacturer Data 作成
  uint8_t mfgData[20] = {
    0x49, 0x53,       // magic "IS"
    0x01,             // version
    0x01,             // productType (is01)
    0x07,             // flags
    0x11, 0x22, 0x33, 0x44, 0x55, 0x66,  // staMac (fake)
    0x00, 0x00, 0x00, 0x00,  // bootCount
    0xE6, 0x09,       // temp = 2534 (25.34C)
    0x70, 0x19,       // hum = 6512 (65.12%)
    0x50              // batt = 80%
  };

  NimBLEAdvertisementData advData;
  advData.setManufacturerData(std::string((char*)mfgData, 20));
  pAdv->setAdvertisementData(advData);
  pAdv->start();

  Serial.println("is01 emulator advertising...");
}

void loop() {
  delay(1000);
}
```

## 8. チェックリスト

### コンパイル
- [ ] エラーなしでコンパイル完了
- [ ] メモリ使用量が許容範囲内

### 起動
- [ ] シリアルにブートログが出る
- [ ] OLEDに起動画面が出る
- [ ] WiFi接続成功（IPが表示される）

### BLE受信
- [ ] ISMSv1ペイロードを認識
- [ ] lacisIdが正しく再構成される
- [ ] 温湿度値が正しくパースされる

### HTTP送信
- [ ] テストサーバにPOSTが届く
- [ ] JSONフォーマットが仕様通り
- [ ] 送信成功時OLEDに "SEND:OK"
- [ ] 送信失敗時OLEDに "SEND:NG"

### 安定性
- [ ] 10分以上連続動作でクラッシュなし
- [ ] WiFi切断→再接続が正常動作
