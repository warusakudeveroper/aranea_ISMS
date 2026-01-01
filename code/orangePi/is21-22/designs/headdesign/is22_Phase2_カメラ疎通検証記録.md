# IS22 Phase 2: カメラ疎通検証記録

## 検証概要

| 項目 | 内容 |
|------|------|
| 実施日 | 2025-12-31 |
| 検証者 | Claude Code |
| 検証元 | IS22サーバー (192.168.125.246) |
| 対象ネットワーク | 192.168.96.0/24, 192.168.97.0/24, 192.168.125.0/24, 192.168.126.0/24 |

---

## 1. 検証目的

1. ネットワーク上のカメラ候補デバイスの検出
2. 各検出フェーズで得られる情報の記録
3. クレデンシャル未設定時でも判別可能な情報の特定
4. IS22カメラスキャン機能の実装に必要なデータ収集

---

## 2. 実行した検証手順

### Phase 0: ARP + MAC + ベンダー特定 (192.168.125.0/24のみ)

```bash
# 1. 全IPにpingを送信しARPテーブルを更新
for i in $(seq 1 254); do
  ping -c 1 -W 1 192.168.125.$i > /dev/null 2>&1 &
done
wait

# 2. ip neighでARP取得
ip neigh show | grep '192.168.125'

# 3. MACアドレスのOUI(先頭3オクテット)からベンダー特定
```

**取得可能情報**: IP, MACアドレス, ベンダー(OUI)

**制約**: 同一サブネット内のみ有効。VPN越しでは取得不可。

---

### Phase 1: ポートスキャン

```bash
nmap -sT -p 80,443,554,2020,8000,8080,8443,8554 <target_network>
```

**スキャンポート**:
| ポート | プロトコル | 用途 |
|--------|-----------|------|
| 80 | HTTP | Web管理画面 |
| 443 | HTTPS | セキュアWeb管理画面 |
| 554 | RTSP | 映像ストリーム |
| 2020 | ONVIF | カメラ制御(Tapo等) |
| 8000 | HTTP-ALT | NVR管理等 |
| 8080 | HTTP-PROXY | 代替Web |
| 8443 | HTTPS-ALT | 代替セキュアWeb |
| 8554 | RTSP-ALT | 代替RTSP |

**取得可能情報**: 各ポートの状態 (open/closed/filtered)

---

### Phase 2: プロトコル別プローブ

#### ONVIF (ポート2020)
```bash
curl -s --max-time 2 -X POST "http://<ip>:2020/onvif/device_service" \
  -H 'Content-Type: application/soap+xml' \
  -d '<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
<s:Body><GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/>
</s:Body></s:Envelope>'
```

**認証**: 不要 (GetSystemDateAndTimeは認証不要API)

**取得可能情報**: デバイス時刻、ONVIF対応の有無

#### RTSP OPTIONS
```bash
echo -e 'OPTIONS rtsp://<ip>/ RTSP/1.0\r\nCSeq: 1\r\n\r\n' | nc -q1 <ip> 554
```

**認証**: 不要 (OPTIONSは認証不要)

**取得可能情報**: RTSP対応の有無、サポートメソッド

#### HTTPS HEAD
```bash
curl -s -k --max-time 2 -I "https://<ip>/"
```

**取得可能情報**: HTTPステータス、サーバー情報

---

### Phase 3: RTSP認証テスト

```bash
ffprobe -v quiet -show_entries stream=codec_name \
  "rtsp://<user>:<pass>@<ip>:554/stream1"
```

**認証**: 必要

**取得可能情報**: コーデック情報、ストリーム有無

---

## 3. 検証結果

### 3.1 192.168.96.0/24 (市山水産 fid:9000)

| IP | 80 | 443 | 554 | 2020 | 8443 | ONVIF | RTSP OPTIONS | HTTPS応答 | RTSP認証(ismscam) | MAC/Vendor |
|----|:--:|:---:|:---:|:----:|:----:|:-----:|:------------:|:---------:|:-----------------:|------------|
| .1 | o | o | c | c | c | - | - | - | - | - |
| .2 | o | o | c | c | c | - | - | - | - | - |
| .4 | o | o | c | c | c | - | - | - | - | - |
| .5 | o | o | c | c | c | - | - | - | - | - |
| .6 | o | o | c | c | c | - | - | - | - | - |
| .7 | o | o | c | c | c | - | - | - | - | - |
| .8 | o | o | c | c | c | - | - | - | - | - |
| .36 | o | o | f | f | f | - | - | - | - | - |
| .37 | o | o | f | f | f | - | - | - | - | - |
| .38 | o | o | f | f | f | - | - | - | - | - |
| .39 | o | o | f | f | f | - | - | - | - | - |
| .49 | o | c | c | c | c | - | - | - | - | - |
| .79 | o | c | c | c | c | - | - | - | - | - |
| .80 | c | o | c | c | o | NG | NG | 405 | - | - |
| .81 | c | o | c | c | o | NG | NG | 405 | - | - |
| .82 | c | o | c | c | o | NG | NG | 405 | - | - |
| .83 | c | o | o | o | o | OK | OK | 405 | OK | - |
| .85 | c | o | o | o | c | OK | OK | 405 | OK | - |
| .86 | c | o | c | c | c | NG | NG | 405 | - | - |
| .88 | c | o | c | c | c | NG | NG | 405 | - | - |
| .89 | c | o | c | c | c | NG | NG | 405 | - | - |
| .90 | c | o | c | c | c | NG | NG | 405 | - | - |
| .91 | c | o | c | c | c | NG | NG | 405 | - | - |
| .201 | c | c | c | c | c | - | - | - | - | - |
| .202 | c | c | c | c | c | - | - | - | - | - |

**192.168.97.0/24**: 0 hosts up (スキャン実施済み、応答なし)

---

### 3.2 192.168.125.0/24 (HALE京都丹波口 fid:0150)

| IP | MAC | Vendor | 80 | 443 | 554 | 2020 | 8443 | ONVIF | RTSP | HTTPS | RTSP認証 |
|----|-----|--------|:--:|:---:|:---:|:----:|:----:|:-----:|:----:|:-----:|:--------:|
| .1 | 30:de:4b:* | TP-Link | o | o | c | c | c | - | - | - | - |
| .9 | 8c:c8:4b:* | Somfy | o | o | c | c | c | - | - | - | - |
| .10 | ac:15:a2:* | TP-Link(VIGI) | o | o | c | c | o | NG | NG | 200 | - |
| .12 | d8:07:b6:* | TP-Link | f | o | o | o | f | OK | OK | 200 | NG |
| .17 | 3c:84:6a:* | TP-Link | f | o | o | o | f | OK | OK | 200 | NG |
| .41 | e4:65:b8:* | HP | f | f | f | f | f | - | - | - | - |
| .42 | a8:6e:84:* | TP-Link | o | o | c | c | c | NG | NG | 405 | - |
| .43 | a8:6e:84:* | TP-Link | o | o | c | c | c | NG | NG | 405 | - |
| .44 | f0:09:0d:* | TP-Link | o | o | c | c | c | NG | NG | 405 | - |
| .45 | 9c:53:22:* | TP-Link | c | o | o | o | o | OK | OK | 405 | NG |
| .47 | 78:24:af:* | ASUSTek | o | c | c | c | c | - | - | - | - |
| .48 | 04:d4:c4:* | ASUSTek | o | c | c | c | c | - | - | - | - |
| .50 | d8:44:89:* | Extreme | o | f | f | f | f | - | - | - | - |
| .62 | 3c:84:6a:* | TP-Link | f | o | o | o | f | OK | OK | 200 | NG |
| .66 | 6c:c8:40:* | TP-Link(Tapo) | o | c | c | c | c | NG | NG | - | - |
| .78 | a8:42:a1:* | TP-Link | c | o | o | o | c | OK | OK | 405 | NG |
| .79 | dc:62:79:* | Google | c | o | c | c | o | NG | NG | 405 | - |
| .80 | 6c:c8:40:* | TP-Link(Tapo) | o | c | c | c | c | NG | NG | - | - |
| .91 | 3c:8d:20:* | Google | - | - | - | - | - | - | - | - | - |

---

### 3.3 192.168.126.0/24 (HALE京都東寺 fid:0151)

| IP | 80 | 443 | 554 | 2020 | 8443 | ONVIF | RTSP | HTTPS | RTSP認証(halecam) |
|----|:--:|:---:|:---:|:----:|:----:|:-----:|:----:|:-----:|:-----------------:|
| .1 | o | o | c | c | c | - | - | - | - |
| .4 | o | c | c | c | c | - | - | - | - |
| .23 | f | o | o | o | f | OK | OK | 200 | OK |
| .24 | f | o | o | o | f | OK | OK | 200 | NG |
| .25 | f | o | o | o | f | OK | OK | 200 | NG |
| .27 | f | o | o | o | f | OK | OK | 200 | OK |
| .28 | f | f | f | f | f | OK | OK | 200 | OK |
| .30 | c | o | o | o | o | OK | OK | 405 | NG |
| .37 | o | c | c | c | c | - | - | - | - |
| .38 | o | c | c | c | c | - | - | - | - |
| .39 | o | c | c | c | c | - | - | - | - |
| .102 | o | c | c | c | c | - | - | - | - |
| .103 | o | o | f | f | f | - | - | - | - |
| .114 | o | f | f | f | f | - | - | - | - |
| .118 | o | c | c | c | c | - | - | - | - |
| .120 | f | o | o | o | f | OK | OK | 200 | NG |
| .124 | o | o | o | o | o | OK | OK | 200 | NG |
| .125 | o | c | c | c | c | - | - | - | - |
| .127 | c | o | o | o | o | OK | OK | 405 | NG |
| .131 | o | c | c | c | c | - | - | - | - |

---

## 4. 凡例

| 記号 | 意味 |
|------|------|
| o | open |
| c | closed |
| f | filtered |
| OK | 応答あり |
| NG | 応答なし/タイムアウト |
| 200 | HTTP 200 OK |
| 405 | HTTP 405 Method Not Allowed |
| - | 未実施/対象外 |

---

## 5. 検出フェーズ別統計

| フェーズ | 検出条件 | 96.x | 125.x | 126.x | 計 |
|---------|---------|:----:|:-----:|:-----:|:--:|
| Phase 0 | ARP応答あり | N/A | 50 | N/A | 50 |
| Phase 1 | いずれかのポートopen | 25 | 31 | 23 | 79 |
| Phase 1a | 554 open | 2 | 5 | 8 | 15 |
| Phase 1b | 2020 open | 2 | 5 | 8 | 15 |
| Phase 1c | 443 open, 554/2020 closed | 8 | 3 | 0 | 11 |
| Phase 2 | ONVIF応答あり | 2 | 5 | 9 | 16 |
| Phase 2 | RTSP OPTIONS応答あり | 2 | 5 | 9 | 16 |
| Phase 3 | RTSP認証成功 | 2 | 0 | 3 | 5 |

---

## 6. 検証で判明した事実

### 6.1 ポートパターン

| パターン | 特徴 | 該当例 |
|---------|------|--------|
| 554+2020 open | ONVIF対応カメラ (Tapo等) | 96.83, 96.85, 125.12, 125.17, 126.23 等 |
| 443+8443 open, 554/2020 closed | Google Nest等の可能性 | 96.80, 96.81, 96.82, 125.79 |
| 443 only open | 未特定デバイス | 96.86, 96.88-91 |
| 80+443+8000+8443 open | VIGI NVR | 125.10 |
| 80+443 open, Server: TP-LINK HTTPD | TP-Link製ネットワーク機器 | 125.42, 125.43, 125.44 |

### 6.2 認証状況

| ネットワーク | 使用クレデンシャル | 成功数 | 失敗数 |
|-------------|-------------------|:------:|:------:|
| 192.168.96.x | ismscam:isms12345@ | 2 | 0 |
| 192.168.125.x | halecam:hale0083 | 0 | 5 |
| 192.168.126.x | halecam:hale0083 | 3 | 6 |

### 6.3 クレデンシャル未設定でも取得可能な情報

| フェーズ | 取得可能情報 | 認証要否 |
|---------|------------|:--------:|
| Phase 0 | IP, MAC, ベンダー | 不要 |
| Phase 1 | 開放ポート一覧 | 不要 |
| Phase 2 | ONVIF対応有無、デバイス時刻 | 不要 |
| Phase 2 | RTSP対応有無 | 不要 |
| Phase 2 | HTTPステータス | 不要 |
| Phase 3 | ストリーム情報、コーデック | **必要** |

---

## 7. 実装への示唆

### 7.1 IpcamScan実装方針

1. **Phase 1 (ポートスキャン)** で全候補を検出
   - 554 or 2020 open → ONVIF/RTSP候補
   - 443+8443 open, 554/2020 closed → Google Nest候補
   - nmap使用、並列実行で高速化

2. **Phase 2 (プロトコルプローブ)** で詳細情報取得
   - ONVIF GetSystemDateAndTime (認証不要)
   - RTSP OPTIONS (認証不要)
   - デバイス存在確認 + ONVIF対応判定

3. **Phase 3 (認証テスト)** は管理者入力後に実行
   - UI上で「検出済み・未認証」状態を表示
   - 管理者がクレデンシャル入力 → 即時テスト

### 7.2 ベンダー特定の活用

| OUI | ベンダー | 推定デバイス |
|-----|---------|------------|
| 6C:C8:40 | TP-Link | Tapoカメラ |
| 3C:84:6A | TP-Link | Tapoカメラ |
| 9C:53:22 | TP-Link | Tapoカメラ |
| A8:42:A1 | TP-Link | Tapoカメラ |
| D8:07:B6 | TP-Link | Tapoカメラ |
| AC:15:A2 | TP-Link | VIGI NVR |
| DC:62:79 | Google | Nest Doorbell/Cam |
| 3C:8D:20 | Google | Nest Doorbell/Cam |

---

## 8. 未解決事項

| 項目 | 状態 | 備考 |
|------|------|------|
| 192.168.96.x 443+8443デバイスの正体 | 未特定 | Google Nest候補だが断定不可 |
| 192.168.125.x カメラクレデンシャル | 未取得 | halecam:hale0083で認証失敗 |
| VIGI NVR (125.10) のRTSP | 未検証 | 554 closed、API経由の可能性 |
| Discovery プロトコル | 非対応確認済 | WS-Discovery, SSDP, mDNS いずれも応答なし |

---

## 9. 関連ドキュメント

- `is22_Camserver実装指示.md` - 実装指示書
- `is22_Camserver仕様案(基本設計書草案改訂版).md` - 基本設計書
- `is22_02_collector設計.md` - Collector設計 (IpcamScan含む)

---

## 更新履歴

| 日付 | 更新者 | 内容 |
|------|--------|------|
| 2025-12-31 | Claude Code | 初版作成 |
