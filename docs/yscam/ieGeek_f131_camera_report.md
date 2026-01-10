# ieGeek f131 カメラ調査報告書

**調査日**: 2026-01-07
**対象機器**: ieGeek IPカメラ (Model: f131)
**IPアドレス**: 192.168.68.50
**調査者**: Claude Code

---

## 1. 概要

安田様戸建に設置されているieGeek f131カメラについて、ローカルネットワーク経由でのRTSP/ONVIF/HTTPストリーム取得の可否を調査した。

## 2. 機器情報

| 項目 | 値 |
|------|-----|
| メーカー | ieGeek (https://jp.iegeek.com/) |
| モデル | f131 |
| ファームウェア | v5.1.10.1906121205 |
| Device ID | 1jfiegbqrbh6q |
| MACアドレス | ae:ca:06:11:09:ff |
| カメラ名称 | 安田様戸建 |
| Webサーバー | MWS 0.01 |
| プラットフォーム | MIPC (クラウドベース) |

## 3. 認証情報

| 項目 | 値 |
|------|-----|
| ユーザー名 | `1jfiegbqrbh6q` |
| パスワード | `kinuya130` |
| Webログイン | **成功** |

## 4. ネットワーク調査結果

### 4.1 ポートスキャン結果

| ポート | 状態 | 用途 |
|--------|------|------|
| 80 | **OPEN** | Webインターフェース |
| 443 | CLOSED | HTTPS |
| 554 | CLOSED | RTSP |
| 8080 | CLOSED | 代替HTTP/ONVIF |
| 8554 | CLOSED | 代替RTSP |
| 8899 | CLOSED | MIPC ONVIF |

### 4.2 RTSP接続テスト

```
rtsp://1jfiegbqrbh6q:kinuya130@192.168.68.50:554/11    → 接続拒否
rtsp://1jfiegbqrbh6q:kinuya130@192.168.68.50:554/12    → 接続拒否
rtsp://1jfiegbqrbh6q:kinuya130@192.168.68.50:554/1     → 接続拒否
rtsp://1jfiegbqrbh6q:kinuya130@192.168.68.50:554/stream1 → 接続拒否
```

**結果**: RTSPポート554が閉鎖されており、すべて失敗

### 4.3 ONVIF接続テスト

```
http://192.168.68.50:80/onvif/device_service   → 応答なし
http://192.168.68.50:8080/onvif/device_service → ポート閉鎖
http://192.168.68.50:8899/onvif/device_service → ポート閉鎖
```

**結果**: ONVIFサービスは無効

### 4.4 HTTPストリーム/スナップショットテスト

| エンドポイント | 結果 |
|----------------|------|
| `/tmpfs/auto.jpg` | 404 (0バイト) |
| `/snap.jpg` | 404 (0バイト) |
| `/videostream.cgi?user=...&pwd=...` | 404 |
| `/live.flv` | 404 (Content-Type: video/x-flv) |
| `/live/stream.m3u8` | 404 (Content-Type: application/vnd.apple.mpegurl) |
| `/cgi-bin/hi3510/snap.cgi` | 404 |
| `/mjpeg.cgi` | 404 |

**結果**: HTTPストリームエンドポイントは存在しない

## 5. Webインターフェース機能

Webインターフェース（http://192.168.68.50/）では以下の機能が確認できた：

### 利用可能な機能
- ライブビュー表示（WebSocket/P2P経由）
- PTZ操作（パン・チルト・ズーム）
- 画質調整（Brightness, Contrast, Saturation, Sharpness）
- 映像反転（Flip）
- 録画再生（Playback）
- 各種設定変更

### 設定項目
- Nickname（カメラ名）
- Admin/Guest パスワード
- Network設定（DHCP/Static）
- OSD設定
- SDカード管理
- モーション検知
- スケジュール録画
- 日時設定
- システム（ファームウェア更新、リセット）

## 6. 結論

### 6.1 ローカルストリーム取得: **不可**

このieGeek f131カメラは**MIPCクラウド専用**設計であり、以下の理由によりローカルネットワークでの直接ストリーム取得ができない：

1. **RTSPポートが閉鎖**: ファームウェアレベルで無効化
2. **ONVIFサービスなし**: デバイスディスカバリ不可
3. **HTTPストリームなし**: MJPEG/スナップショットエンドポイント未実装
4. **映像配信方式**: WebSocket/P2Pベースのクラウド経由のみ

### 6.2 go2rtc/NVR統合: **不可**

このカメラはgo2rtcやis22システムへの統合には適さない。

## 7. 推奨事項

### 7.1 短期対応
1. **ieGeek Camアプリ**でRTSP設定の有無を確認
   - 一部機種ではアプリ内でRTSP有効化オプションが存在

2. **ファームウェア更新確認**
   - 新しいファームウェアでRTSP機能が追加される可能性

### 7.2 長期対応（推奨）
RTSP/ONVIFネイティブ対応カメラへの交換を推奨：

| メーカー | モデル例 | 特徴 |
|----------|----------|------|
| TP-Link | Tapo C200/C210 | 安価、RTSP対応 |
| Reolink | E1 Pro | ONVIF対応、高画質 |
| Hikvision | DS-2CD系 | 業務用、フル機能 |
| Dahua | IPC-HDW系 | 業務用、ONVIF標準 |

## 8. 参考資料

- [iSpy Connect - ieGeek Camera Setup](https://www.ispyconnect.com/camera/iegeek)
- [ieGeek公式サイト](https://jp.iegeek.com/)
- [ZoneMinder Forums - ieGeek discussion](https://forums.zoneminder.com/viewtopic.php?t=32254)

---

## 付録: テストコマンド一覧

### ポートスキャン
```bash
nc -zv -w 3 192.168.68.50 80 554 8080 8554 8899
```

### RTSP接続テスト
```bash
ffprobe -v quiet "rtsp://USER:PASS@192.168.68.50:554/11"
```

### ONVIF probe
```bash
curl -s "http://192.168.68.50/onvif/device_service" \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>'
```

### HTTPスナップショットテスト
```bash
curl -s -I "http://192.168.68.50/tmpfs/auto.jpg"
curl -s -I "http://192.168.68.50/snap.jpg?usr=USER&pwd=PASS"
```
