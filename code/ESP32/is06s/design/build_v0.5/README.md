# IS06S Build v0.5

**ビルド日時**: 2026-01-25 08:20 JST
**ファームウェア**: 1.0.0
**UI バージョン**: 1.6.0
**パーティション**: min_spiffs (OTA対応)

---

## ビルド情報

```
フラッシュ使用: 1,455,953 / 1,966,080 バイト (74%)
RAM使用: 54,000 / 327,680 バイト (16%)
FQBN: esp32:esp32:esp32:PartitionScheme=min_spiffs
```

---

## ファイル一覧

| ファイル | サイズ | 用途 |
|----------|--------|------|
| is06s.ino.bin | 1.4MB | OTA更新用ファームウェア |
| is06s.ino.merged.bin | 4.0MB | 初期書き込み用（bootloader含む） |
| is06s.ino.bootloader.bin | 24KB | ブートローダー単体 |
| is06s.ino.partitions.bin | 3KB | パーティションテーブル |
| is06s.ino.elf | 18MB | デバッグ用ELF |

---

## インストール方法

### 1. OTA更新（既存デバイス）

HTTP経由でIPアドレス直接指定:
```bash
# MCP tool使用
mcp__mcp-arduino-esp32__ota_update \
  device_ip="192.168.x.x" \
  firmware_path="is06s.ino.bin"

# curl使用
curl -X POST http://192.168.x.x/api/ota/update \
  -F "firmware=@is06s.ino.bin"
```

### 2. 初期書き込み（新規デバイス）

USBシリアル経由:
```bash
# esptool.py使用
esptool.py --chip esp32 --port /dev/ttyUSB0 --baud 921600 \
  write_flash 0x0 is06s.ino.merged.bin

# MCP tool使用
mcp__mcp-arduino-esp32__upload \
  sketch_path="." \
  port="/dev/ttyUSB0" \
  build_path="/path/to/build_v0.5"
```

---

## VPN越しOTA

L3ルーティング可能な環境であれば、mDNSに依存せずIPアドレス直接指定でOTA更新可能。

```bash
# VPN越し（IP直接指定）
curl -X POST http://10.0.0.x/api/ota/update \
  -F "firmware=@is06s.ino.bin"
```

---

## 変更履歴

- Designer Review #1 全14項目対応完了
- OLED通知5秒タイムアウト修正
- MQTT接続状態報告修正
- rid (roomID) 実装
- PIN Control行形式レイアウト
- PIN Settings条件付きグレーアウト
