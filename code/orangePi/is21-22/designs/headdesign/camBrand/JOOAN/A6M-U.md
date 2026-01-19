JOOAN A6M-U

操作用アプリ(iOS or Android) CAM720
アプリベンダー WEIZHI
testアカウント:webadmin@mijeos.com
pass:test98765@

MAC 48:D0:1C:1F:2F:6D
IP=192.168.77.23
カメラ名:jooan_test
セキュリティ設定/
    セキュリティコード　admin123
    →パスワード変更
    test12345

## 実機検証結果（2026-01-17）✅

### OUI情報
- OUIプレフィックス: 48:D0:1C
- ベンダー: **AltoBeam Inc.**（北京）
- 登録日: 2025-06-02

### ポートスキャン結果
| Port | State | Service |
|------|-------|---------|
| 80   | open  | HTTP    |
| 443  | open  | HTTPS   |
| 554  | open  | RTSP    |
| 8899 | open  | **ONVIF** |

### RTSP情報 ✅動作確認済み

**認証情報:**
- ユーザー名: `admin` (小文字) ★`Admin`(大文字A)は401エラー
- パスワード: `test12345`

**メインストリーム (2304x1296, 15fps)**
```
rtsp://admin:test12345@192.168.77.23:554/live/ch00_0
```

**サブストリーム (640x360, 15fps)**
```
rtsp://admin:test12345@192.168.77.23:554/live/ch00_1
```

### ONVIF情報 ✅動作確認済み

**重要: ポート8899を使用**
```
http://192.168.77.23:8899/onvif/device_service
```

検出サービス: Device, Media, PTZ, Imaging, Analytics, Events, DeviceIO

### 詳細
→ IS22_REGISTRATION_GUIDE.md 参照
