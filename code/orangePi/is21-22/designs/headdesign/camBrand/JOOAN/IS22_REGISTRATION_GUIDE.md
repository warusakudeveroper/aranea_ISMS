# JOOAN A6M-U IS22登録ガイド

## 調査日時
2026-01-17（実機検証完了）

## デバイス基本情報

| 項目 | 値 |
|------|-----|
| モデル | JOOAN A6M-U |
| MAC | 48:D0:1C:1F:2F:6D |
| OUI | AltoBeam Inc. (北京) |
| テストIP | 192.168.77.23 |
| 管理アプリ | CAM720 (WEIZHI) |

## ポートスキャン結果

```
PORT      STATE  SERVICE
80/tcp    open   HTTP (Web UI)
443/tcp   open   HTTPS
554/tcp   open   RTSP
8899/tcp  open   ONVIF ★重要
```

## RTSP接続情報（実機検証済み）

### 認証情報
| 項目 | 値 |
|------|-----|
| ユーザー名 | `admin` (小文字) ★`Admin`は認証失敗 |
| パスワード | `test12345`（CAM720で設定） |
| 認証方式 | Basic認証 |
| トランスポート | TCP推奨 |

### 動作確認済みRTSP URL

**メインストリーム（高解像度）**
```
rtsp://admin:test12345@192.168.77.23:554/live/ch00_0
```
| 項目 | 値 |
|------|-----|
| コーデック | H.264 (High Profile) |
| 解像度 | **2304x1296** (約3MP) |
| フレームレート | 15fps |
| 音声 | PCM A-law (G.711), 8000Hz, mono |

**サブストリーム（低解像度）**
```
rtsp://admin:test12345@192.168.77.23:554/live/ch00_1
```
| 項目 | 値 |
|------|-----|
| コーデック | H.264 (High Profile) |
| 解像度 | **640x360** |
| フレームレート | 15fps |
| 音声 | PCM A-law (G.711), 8000Hz, mono |

### RTSPパス優先順位

| 優先度 | パス | ステータス |
|--------|------|------------|
| 1 | `/live/ch00_0` | ✅ **動作確認済み**（メイン） |
| 2 | `/live/ch00_1` | ✅ **動作確認済み**（サブ） |
| 3 | `/onvif1` | 未検証 |
| 4 | `/11` | 未検証 |

## ONVIF接続情報（実機検証済み）

### 重要: ONVIFはポート8899

```
ONVIFエンドポイント: http://192.168.77.23:8899/onvif/device_service
```
※ポート80ではタイムアウト、**8899で動作確認**

### 検出されたONVIFサービス

| サービス | エンドポイント |
|----------|----------------|
| Device | http://192.168.77.23:8899/onvif/device_service |
| Media | http://192.168.77.23:8899/onvif/Media |
| PTZ | http://192.168.77.23:8899/onvif/Ptz |
| Imaging | http://192.168.77.23:8899/onvif/Imaging |
| Analytics | http://192.168.77.23:8899/onvif/Analytics |
| Events | http://192.168.77.23:8899/onvif/Events |
| DeviceIO | http://192.168.77.23:8899/onvif/DeviceIO |

### ONVIFストリーミング対応

| 機能 | 対応 |
|------|------|
| RTPMulticast | ✅ |
| RTP_TCP | ✅ |
| RTP_RTSP_TCP | ✅ |

### ONVIFバージョン
- 2.0, 2.60, 16.12, 19.12

## IS22への登録手順（試験段階）

> ⚠️ 現在試験段階のため、実際の登録は保留

### 1. RTSPパス検証コマンド

```bash
# is22サーバー(192.168.125.246)で実行
ffprobe -rtsp_transport tcp -v error -show_streams \
  "rtsp://admin:test12345@192.168.77.23:554/live/ch00_0"
```

### 2. ONVIF検証コマンド

```bash
curl -sv --max-time 5 -X POST \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetCapabilities xmlns="http://www.onvif.org/ver10/device/wsdl"><Category>All</Category></GetCapabilities></s:Body></s:Envelope>' \
  http://192.168.77.23:8899/onvif/device_service
```

### 3. IS22 API登録例（将来用）

```bash
curl -X POST "http://192.168.125.246:8080/api/cameras" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "jooan_test",
    "rtsp_url": "rtsp://admin:test12345@192.168.77.23:554/live/ch00_0",
    "rtsp_sub_url": "rtsp://admin:test12345@192.168.77.23:554/live/ch00_1",
    "brand": "JOOAN",
    "model": "A6M-U",
    "mac_address": "48:D0:1C:1F:2F:6D",
    "onvif_port": 8899
  }'
```

## OUI情報

| 項目 | 値 |
|------|-----|
| OUIプレフィックス | 48:D0:1C |
| ベンダー | AltoBeam Inc. |
| 所在地 | B808 Tsinghua Tongfang Hi-Tech Plaza, Haidian, Beijing 100083, CN |
| 登録日 | 2025-06-02 |
| タイプ | MA-L (Mac Address Block Large) |

## Webインターフェース

- URL: `http://192.168.77.23/home.htm`
- 認証: admin / test12345
- ポリシー: qacloud.com.cn（中国系ファームウェア）

## トラブルシューティング

### RTSPが401 Unauthorizedになる場合

1. **ユーザー名を確認**: `admin`（小文字）を使用、`Admin`は不可
2. **パスワード確認**: CAM720アプリで設定したパスワード
3. **トランスポート**: `-rtsp_transport tcp` を使用

### ONVIFが応答しない場合

1. **ポート確認**: **8899**を使用（80ではない）
2. **SOAPリクエスト**: GETではなくPOSTで送信
3. **Content-Type**: `application/soap+xml` を指定

## 参考リンク

- [iSpy JOOAN Setup](https://www.ispyconnect.com/camera/jooan)
- [Camlytics JOOAN Setup](https://camlytics.com/camera/jooan)
- [Home Assistant JOOAN Integration](https://community.home-assistant.io/t/jooan-ip-ptz-camera-integration/789234)
- [MAC Lookup](https://maclookup.app/)

## 備考

- AltoBeamチップセット採用（中国製）
- CAM720アプリ（WEIZHI社）との互換性あり
- **ONVIF完全対応（ポート8899）**
- PTZ対応（ONVIFサービス検出済み）
- CAM720アプリではパスワードのみ設定可能（ユーザー名は固定`admin`）
