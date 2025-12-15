# OTA (Over-The-Air) アップデート手順

ESP32デバイスへのネットワーク経由ファームウェア更新手順。

## 前提条件

- デバイスがWi-Fi接続済み
- PCとデバイスが同一ネットワーク上にある
- Python3がインストール済み
- espota.pyが利用可能（Arduino ESP32コアに含まれる）

## espota.py の場所

```
~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py
```

※バージョン番号（3.2.1）はインストール済みのESP32コアバージョンに依存

## OTAアップデートコマンド

### 基本形式

```bash
python3 ~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i <デバイスIP> -p 3232 -f <ファームウェア.bin>
```

### 実行例

```bash
# is04へのOTAアップデート
python3 ~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i 192.168.97.113 -p 3232 -f /tmp/is04_build/is04.ino.bin

# is02へのOTAアップデート
python3 ~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i 192.168.97.64 -p 3232 -f /tmp/is02_build/is02.ino.bin
```

## パラメータ説明

| パラメータ | 説明 |
|-----------|------|
| `-i` | デバイスのIPアドレス |
| `-p` | OTAポート（デフォルト: 3232） |
| `-f` | アップロードするファームウェアファイル |
| `-a` | 認証パスワード（設定している場合） |
| `-r` | リトライ回数 |

## デバイスIP確認方法

### シリアルモニタから

```
[WIFI] IP: 192.168.97.113
```

### mDNSホスト名

各デバイスは `{デバイスタイプ}-{MACアドレス下4桁}.local` でアクセス可能：

- is02: `is02-XXXX.local`
- is04: `is04-XXXX.local`
- is05: `is05-XXXX.local`

### ネットワークスキャン

```bash
# Arduino CLIでネットワーク上のESP32を検出
arduino-cli board list
```

## OTA対応確認

シリアルモニタで以下のログが出力されていればOTA対応済み：

```
[OTA] Ready: is04-A2D4.local
```

## トラブルシューティング

### "No response from device" エラー

1. デバイスIPが正しいか確認
2. PCとデバイスが同一ネットワークか確認
3. ファイアウォールがUDP 3232をブロックしていないか確認
4. デバイスを再起動して再試行

### アップロード途中で失敗

1. ファームウェアサイズがパーティションに収まるか確認
2. デバイスの電源が安定しているか確認
3. Wi-Fi信号強度を確認（RSSIが-70dBm以下は不安定）

### arduino-cliでのOTA

arduino-cliは非インタラクティブモードでネットワークアップロードをサポートしていないため、`espota.py`を直接使用すること。

## ビルドからOTAまでの一連の流れ

```bash
# 1. コンパイル
arduino-cli compile --fqbn esp32:esp32:esp32 \
  --build-path /tmp/is04_build \
  /path/to/is04/is04.ino

# 2. OTAアップロード
python3 ~/Library/Arduino15/packages/esp32/hardware/esp32/3.2.1/tools/espota.py \
  -i 192.168.97.113 -p 3232 -f /tmp/is04_build/is04.ino.bin

# 3. 動作確認
curl -s http://192.168.97.113/status
```

## 注意事項

- OTAアップデート中はデバイスの電源を切らないこと
- アップデート完了後、デバイスは自動的に再起動する
- is01（電池式BLEセンサー）はOTA非対応（DeepSleep運用のため）
