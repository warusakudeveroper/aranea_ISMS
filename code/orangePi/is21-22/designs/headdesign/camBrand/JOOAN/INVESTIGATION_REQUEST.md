# JOOAN A6M-U RTSP/ONVIF調査依頼

## 依頼日
2026-01-17

## 依頼元
IS22カメラ管理システム開発チーム

## 対象デバイス

| 項目 | 値 |
|------|-----|
| メーカー | JOOAN |
| モデル | A6M-U |
| MAC | 48:D0:1C:1F:2F:6D |
| OUI | AltoBeam Inc.（北京） |
| 管理アプリ | CAM720 (WEIZHI社) |
| テストIP | 192.168.77.23 |

## 調査目的

IS22（RTSPカメラ総合管理サーバー）にJOOANブランドのカメラを登録するため、RTSPストリームの正確なURL構築方法とONVIF対応状況を確認したい。

## 現状の調査結果

### ポートスキャン
```
80/tcp    open   HTTP
443/tcp   open   HTTPS
554/tcp   open   RTSP
8899/tcp  open   (用途不明)
```

### RTSP接続テスト
- RTSPサーバー応答: **200 OK**
- 対応メソッド: OPTIONS, DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE, GET_PARAMETER, SET_PARAMETER

### ONVIF接続テスト
- `/onvif/device_service` へのSOAPリクエスト: **タイムアウト**
- 標準ONVIFエンドポイント未対応の可能性

### Webインターフェース
- URL: `http://192.168.77.23/home.htm`
- ログインページ表示確認
- ポリシーリンク: qacloud.com.cn（中国系ファームウェア）

## 調査依頼事項

### 1. RTSPストリームURL構築方法

**質問:**
JOOAN A6M-U（AltoBeamチップセット）の正確なRTSPパスを教えてください。

**試行済みパス（未検証）:**
- `/onvif1`
- `/live/ch00_0` (メインストリーム)
- `/live/ch00_1` (サブストリーム)
- `/11`
- `/stream1`

**確認したい項目:**
- メインストリーム（高解像度）のパス
- サブストリーム（低解像度）のパス
- 認証方式（Basic認証 / Digest認証）
- トランスポート（TCP / UDP）

### 2. 認証クレデンシャル

**現状:**
- CAM720アプリからは**パスワードのみ設定可能**
- ユーザー名の設定項目なし

**質問:**
- デフォルトのユーザー名は何か？（`admin`と推測）
- RTSP認証とWeb認証は同一か？
- 認証なしでのRTSPアクセスは可能か？

**現在の設定:**
```
ユーザー名: admin（推測）
パスワード: test12345（CAM720で設定済み）
```

### 3. ONVIF対応状況

**質問:**
- ONVIF対応の有無
- 対応している場合のONVIFエンドポイントパス
- ONVIFプロファイル（Profile S / Profile T等）
- ONVIF認証情報（RTSP認証と共通か）

**確認したいONVIFサービス:**
- デバイスサービス（GetCapabilities）
- メディアサービス（GetProfiles, GetStreamUri）
- PTZサービス（対応している場合）

### 4. ファームウェア/チップセット情報

**確認したい項目:**
- ファームウェアバージョン確認方法
- AltoBeamチップセット固有の設定
- CAM720/WEIZHI以外の管理方法（telnet, SSH等）

## 参考情報

### 類似モデルの情報（iSpy/Camlytics）

| モデル | RTSPパス |
|--------|----------|
| JA-C9C-Y | `/live/ch00_0`, `/live/ch00_1` |
| JA-F11C | `/realmonitor?user=...&channel=0&stream=0.sdp` |
| 一般的 | `/onvif1` |

### AltoBeam Inc.について
- 本社: 北京（清華同方ハイテクプラザ）
- OUI登録: 2025-06-02
- WiFiカメラ向けチップセットメーカー

## 期待する回答

1. **動作確認済みRTSP URL**
   ```
   rtsp://[user]:[pass]@[ip]:554/[path]
   ```

2. **ONVIF接続情報**（対応している場合）
   ```
   ONVIFエンドポイント: http://[ip]/[path]
   認証: [方式]
   ```

3. **IS22への登録に必要な追加情報**

## 連絡先

調査結果は以下のディレクトリに配置してください:
```
code/orangePi/is21-22/designs/headdesign/camBrand/JOOAN/
```

または、LLM共有ディレクトリ:
```
warusakudeveroper/mobes2.0
パス: doc/APPS/Paraclate/LLM/
```
