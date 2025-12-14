# BLE Payload Specification (ISMSv1)

## 概要

is01（温湿度センサー）がBLEアドバタイズで送信するペイロード仕様。
is02/is03はこのペイロードを受信し、lacisIdを再構成してHTTP中継する。

## 重要な設計原則

1. **BLEアドレスのランダム化対策**: BLEアドレスはランダム化されうるため、STA MAC（Wi-Fi MAC）をペイロードに含める
2. **lacisId再構成**: 受信側でSTA MACからlacisIdを再構成する（送信側でlacisId全体を送らない）
3. **識別マジック**: ISMSv1ペイロードを他のBLE広告と区別するためのマジックバイトを使用

## Manufacturer Specific Data 構造（ISMSv1）

| Offset | Size | Field | Type | 値/説明 |
|--------|------|-------|------|---------|
| 0 | 2 | magic | bytes | `0x49 0x53` ("IS") - ISMS識別子 |
| 2 | 1 | version | uint8 | `0x01` - ISMSv1 |
| 3 | 1 | productType | uint8 | `0x01` = is01, `0x02` = is02, ... |
| 4 | 1 | flags | uint8 | bit0: hasTemp, bit1: hasHum, bit2: hasBatt, bit3: hasBootCount |
| 5 | 6 | staMac | bytes | is01のSTA MAC（WiFi MAC）6バイト |
| 11 | 4 | bootCount | uint32_le | 起床回数（optional, flags bit3=1の場合） |
| 15 | 2 | temp_x100 | int16_le | 温度×100（例: 2534 = 25.34℃） |
| 17 | 2 | hum_x100 | uint16_le | 湿度×100（例: 6520 = 65.20%） |
| 19 | 1 | batt_pct | uint8 | バッテリー残量 0-100% |

**合計: 20バイト**（BLE Advertising 31バイト制限内）

## バイト配列例

is01, STA MAC = `AA:BB:CC:DD:EE:FF`, temp=25.34℃, hum=65.20%, batt=80%:

```
49 53 01 01 07 AA BB CC DD EE FF 00 00 00 00 E6 09 70 19 50
│  │  │  │  │  └──────────────┘  └──────────┘ └────┘ └────┘ │
│  │  │  │  │       staMac        bootCount    temp   hum   batt
│  │  │  │  flags (0x07 = hasTemp|hasHum|hasBatt)
│  │  │  productType (0x01 = is01)
│  │  version (0x01)
magic "IS"
```

## lacisId 再構成ロジック（受信側）

```cpp
// 受信したペイロードからlacisIdを再構成
String reconstructLacisId(uint8_t productType, const uint8_t* staMac) {
  char lacisId[21];
  snprintf(lacisId, sizeof(lacisId),
    "3%03d%02X%02X%02X%02X%02X%02X0096",
    productType,
    staMac[0], staMac[1], staMac[2],
    staMac[3], staMac[4], staMac[5]);
  return String(lacisId);
}

// 例: productType=1, staMac={0xAA,0xBB,0xCC,0xDD,0xEE,0xFF}
// → "3001AABBCCDDEEFF0096"
```

## ISMSv1 判定ロジック

```cpp
bool isIsmsV1Payload(const uint8_t* data, size_t len) {
  if (len < 20) return false;
  if (data[0] != 0x49 || data[1] != 0x53) return false;  // "IS"
  if (data[2] != 0x01) return false;  // version 1
  return true;
}
```

## productCode について

- productCode は **"0096" 固定**
- ペイロードには含めない（受信側で固定値として付与）
- これにより1バイト節約

## 注意事項

1. **バイトオーダー**: マルチバイト値はリトルエンディアン（ESP32ネイティブ）
2. **flags フィールド**: 将来の拡張用。現在は bit0-3 を使用
3. **STA MAC の取得**: `esp_read_mac()` または `WiFi.macAddress()` で取得
4. **BLEアドレス**: アドバタイズのBLEアドレスは使用しない（ランダム化対策）
