# IS06S Firmware v0.6

## Build Information
- **Build Date**: 2026-01-26
- **Firmware Size**: 1,476,144 bytes (74% of 1,966,080)
- **RAM Usage**: 54,040 bytes (16% of 327,680)
- **Partition Scheme**: min_spiffs (OTA対応)
- **FQBN**: esp32:esp32:esp32:PartitionScheme=min_spiffs

## Changes from v0.5
- PIN Control UI改善
  - Disabled PINが非表示に
  - Digital Outputにクリック可能ボタン追加
  - PWMにスライダー操作追加（Slow遷移アニメーション対応）
  - data-*属性によるJSエスケープ問題解消
- GPIO18/GPIO5初期化にgpio_reset_pin追加

## Files
| File | Size | Description |
|------|------|-------------|
| is06s.ino.bin | 1.4MB | OTA用ファームウェア |
| is06s.ino.merged.bin | 4MB | フルフラッシュイメージ |
| is06s.ino.bootloader.bin | 24KB | ブートローダー |
| is06s.ino.partitions.bin | 3KB | パーティションテーブル |
| is06s.ino.elf | 19MB | デバッグシンボル付きバイナリ |
| is06s.ino.map | 17MB | リンカマップ |

## OTA Update
```bash
# HTTP OTA (ArduinoMCP)
curl -X POST -F "firmware=@is06s.ino.bin" http://<device_ip>/api/ota/update

# MCP Tool
mcp__mcp-arduino-esp32__ota_update device_ip=<IP> firmware_path=is06s.ino.bin
```
