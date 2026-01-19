# 新規カメラブランド検証手順書

## 前提条件

- IS22サーバー（192.168.125.246）へのSSHアクセス
- 検証対象カメラがネットワーク上で到達可能
- カメラの認証情報（ユーザー名/パスワード）が判明している

---

## Phase 1: 事前情報収集

### 1.1 製品情報の確認

以下の情報を収集してドキュメント化する。

| 項目 | 確認方法 |
|------|---------|
| ブランド名 | 製品パッケージ/マニュアル |
| モデル名 | 製品パッケージ/本体ラベル |
| MACアドレス | 管理アプリ/本体ラベル/ルーター管理画面 |
| IPアドレス | 管理アプリ/ルーター管理画面 |
| 管理アプリ名 | 製品マニュアル |
| ユーザー名/パスワード | 管理アプリで設定したもの |

### 1.2 ネット上の技術情報調査

以下のサイトでRTSP URL等を調査する。

```
調査サイト:
- https://www.ispyconnect.com/camera/{brand}
- https://camlytics.com/camera/{brand}
- https://community.home-assistant.io/
- https://www.reddit.com/r/frigate_nvr/
- https://github.com/{brand}/issues
```

**調査項目:**
- RTSP URLパス（メイン/サブ）
- ONVIFポート（標準: 80/2020、非標準の場合あり）
- 認証方式（Basic/Digest）
- 既知の制限事項

---

## Phase 2: 基本接続テスト

### 2.1 ポートスキャン

```bash
# IS22サーバーで実行
CAMERA_IP="192.168.XXX.XXX"

# 基本ポートスキャン
nmap -sT -p 80,443,554,2020,8080,8443,8554,8899 $CAMERA_IP

# 詳細スキャン（全ポート）
nmap -sT -p 1-10000 --open $CAMERA_IP
```

**確認項目:**
| ポート | 用途 | 必須 |
|--------|------|------|
| 80 | HTTP/ONVIF | △ |
| 443 | HTTPS | △ |
| 554 | RTSP | ✅ |
| 2020 | ONVIF (TP-link) | △ |
| 8899 | ONVIF (NVT系) | △ |

### 2.2 OUI調査

```bash
# MACアドレスからOUIを調査
MAC="XX:XX:XX:XX:XX:XX"
OUI_PREFIX=$(echo $MAC | cut -d: -f1-3 | tr ':' '-')

# オンラインAPI
curl -s "https://api.maclookup.app/v2/macs/$OUI_PREFIX" | jq .

# または maclookup.app で検索
echo "https://maclookup.app/search/result?mac=$OUI_PREFIX"
```

**記録項目:**
- OUIプレフィックス
- ベンダー名
- 登録国/地域

### 2.3 ONVIF接続テスト

```bash
CAMERA_IP="192.168.XXX.XXX"

# ポート80で試行
curl -sv --max-time 5 -X POST \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>' \
  http://$CAMERA_IP:80/onvif/device_service

# ポート2020で試行
curl -sv --max-time 5 -X POST \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>' \
  http://$CAMERA_IP:2020/onvif/device_service

# ポート8899で試行（NVT系）
curl -sv --max-time 5 -X POST \
  -H "Content-Type: application/soap+xml" \
  -d '<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>' \
  http://$CAMERA_IP:8899/onvif/device_service
```

**記録項目:**
- 成功したONVIFポート
- Manufacturer値
- Model値
- FirmwareVersion
- 認証の要否

### 2.4 RTSP接続テスト

```bash
CAMERA_IP="192.168.XXX.XXX"
USER="username"
PASS="password"

# 一般的なRTSPパスを試行
RTSP_PATHS=(
  "/stream1"
  "/stream2"
  "/live/ch00_0"
  "/live/ch00_1"
  "/h264/ch1/main/av_stream"
  "/h264/ch1/sub/av_stream"
  "/Streaming/Channels/101"
  "/Streaming/Channels/102"
  "/cam/realmonitor?channel=1&subtype=0"
  "/cam/realmonitor?channel=1&subtype=1"
)

for path in "${RTSP_PATHS[@]}"; do
  echo "Testing: rtsp://$USER:***@$CAMERA_IP:554$path"
  timeout 5 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width,height,codec_name -of csv=p=0 \
    "rtsp://$USER:$PASS@$CAMERA_IP:554$path" 2>/dev/null && echo "SUCCESS" || echo "FAILED"
  echo ""
done
```

**記録項目:**
- 成功したRTSPパス（メイン/サブ）
- 解像度（メイン/サブ）
- コーデック
- フレームレート

---

## Phase 3: ストレステスト

### 3.1 統合テストスクリプト実行

```bash
# スクリプトをIS22サーバーに配置して実行
scp scripts/camera_stress_test.sh mijeosadmin@192.168.125.246:/tmp/
ssh mijeosadmin@192.168.125.246 'chmod +x /tmp/camera_stress_test.sh'
ssh mijeosadmin@192.168.125.246 '/tmp/camera_stress_test.sh \
  --ip 192.168.XXX.XXX \
  --user username \
  --pass password \
  --main-path /stream1 \
  --sub-path /stream2 \
  --onvif-port 2020'
```

### 3.2 個別テスト（手動実行用）

#### 3.2.1 再接続テスト

```bash
RTSP_URL="rtsp://user:pass@192.168.XXX.XXX:554/stream1"

# 急速再接続（インターバルなし）
echo "=== Rapid Reconnection Test (20 times) ==="
success=0
for i in $(seq 1 20); do
  if timeout 3 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width -of csv=p=0 "$RTSP_URL" 2>/dev/null | grep -qE "^[0-9]+"; then
    success=$((success + 1))
  fi
done
echo "Result: $success/20"

# 0.5秒インターバル
echo "=== Reconnection with 0.5s interval (10 times) ==="
success=0
for i in $(seq 1 10); do
  if timeout 3 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width -of csv=p=0 "$RTSP_URL" 2>/dev/null | grep -qE "^[0-9]+"; then
    success=$((success + 1))
  fi
  sleep 0.5
done
echo "Result: $success/10"

# 1秒インターバル
echo "=== Reconnection with 1s interval (10 times) ==="
success=0
for i in $(seq 1 10); do
  if timeout 3 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width -of csv=p=0 "$RTSP_URL" 2>/dev/null | grep -qE "^[0-9]+"; then
    success=$((success + 1))
  fi
  sleep 1
done
echo "Result: $success/10"

# 2秒インターバル
echo "=== Reconnection with 2s interval (10 times) ==="
success=0
for i in $(seq 1 10); do
  if timeout 3 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width -of csv=p=0 "$RTSP_URL" 2>/dev/null | grep -qE "^[0-9]+"; then
    success=$((success + 1))
  fi
  sleep 2
done
echo "Result: $success/10"
```

#### 3.2.2 並列接続テスト

```bash
RTSP_URL="rtsp://user:pass@192.168.XXX.XXX:554/stream1"

test_concurrent() {
  local count=$1
  echo "=== Testing $count concurrent connections ==="

  for run in 1 2 3; do
    echo "Run $run:"
    pids=""
    for i in $(seq 1 $count); do
      (timeout 5 ffprobe -rtsp_transport tcp -v error \
        -show_entries stream=width -of csv=p=0 "$RTSP_URL" 2>/dev/null | \
        grep -qE "^[0-9]+" && echo "  $i: OK" || echo "  $i: FAIL") &
      pids="$pids $!"
    done
    wait $pids
    sleep 2
  done
}

test_concurrent 2
test_concurrent 3
test_concurrent 5
```

#### 3.2.3 Main+Sub同時テスト

```bash
MAIN_URL="rtsp://user:pass@192.168.XXX.XXX:554/stream1"
SUB_URL="rtsp://user:pass@192.168.XXX.XXX:554/stream2"

echo "=== Main + Sub Simultaneous Test ==="
for run in 1 2 3; do
  echo "Run $run:"
  (timeout 5 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width,height -of csv=p=0 "$MAIN_URL" 2>/dev/null) &
  (timeout 5 ffprobe -rtsp_transport tcp -v error \
    -show_entries stream=width,height -of csv=p=0 "$SUB_URL" 2>/dev/null) &
  wait
  sleep 2
done
```

#### 3.2.4 長時間ストリームテスト

```bash
RTSP_URL="rtsp://user:pass@192.168.XXX.XXX:554/stream1"

echo "=== Long Duration Stream Test (30 seconds) ==="
start=$(date +%s)
timeout 35 ffmpeg -rtsp_transport tcp -i "$RTSP_URL" -t 30 -f null - 2>&1 | tail -5
result=$?
end=$(date +%s)
duration=$((end - start))
echo "Result: $([ $result -eq 0 ] && echo 'SUCCESS' || echo 'FAILED') (ran for ${duration}s)"
```

#### 3.2.5 ONVIFストレステスト

```bash
ONVIF_URL="http://192.168.XXX.XXX:2020/onvif/device_service"
SOAP_REQ='<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>'

echo "=== ONVIF Rapid Requests (20 times) ==="
success=0
for i in $(seq 1 20); do
  response=$(curl -s --max-time 3 -X POST \
    -H "Content-Type: application/soap+xml" \
    -d "$SOAP_REQ" "$ONVIF_URL" 2>/dev/null)
  if echo "$response" | grep -qiE "manufacturer"; then
    success=$((success + 1))
  fi
done
echo "Result: $success/20"
```

---

## Phase 4: 結果分析・設定値決定

### 4.1 判定基準

| テスト項目 | 判定基準 |
|-----------|---------|
| 急速再接続 | 80%以上 → インターバル不要、それ以下 → インターバル必要 |
| 並列接続 | 100%成功した最大数を max_concurrent_streams に設定 |
| Main+Sub同時 | 成功 → 並列可、失敗 → max_concurrent=1 |

### 4.2 Access Absorber設定値決定

テスト結果から以下を決定:

| パラメータ | 決定方法 |
|-----------|---------|
| `max_concurrent_streams` | 並列テストで100%成功した最大数 |
| `min_reconnect_interval_ms` | 100%成功したインターバルの最小値 |
| `require_exclusive_lock` | max_concurrent=1 なら TRUE |

### 4.3 RTSPテンプレート作成

```sql
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, '{BrandName} Standard', '{main_path}', '{sub_path}', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = '{family}';
```

---

## Phase 5: ドキュメント作成

### 5.1 ストレステストレポート作成

`camBrand/{BrandName}/STRESS_TEST_REPORT.md` を作成（テンプレート使用）

### 5.2 チェックリスト完成

`CHECKLIST.md` の全項目を埋める

### 5.3 ブランド統合提案書作成（必要に応じて）

DB投入用SQLとコード変更案を含む提案書を作成

---

## 結果判定マトリクス

| 急速再接続 | 並列接続 | Main+Sub | 推奨設定 |
|-----------|---------|----------|---------|
| ✅ 100% | ✅ 3以上 | ✅ | max=3, interval=0, lock=false |
| ✅ 100% | ❌ 1のみ | ❌ | max=1, interval=0, lock=true, **go2rtc推奨** |
| ❌ <80% | ✅ 3以上 | ✅ | max=3, interval=2000, lock=false |
| ❌ <80% | ❌ 1のみ | ❌ | max=1, interval=2000, lock=true |

---

## トラブルシューティング

### RTSPが401 Unauthorizedになる

1. ユーザー名の大文字小文字を確認（Admin vs admin）
2. パスワードのURL特殊文字をエンコード（@ → %40）
3. Basic認証とDigest認証の両方を試す

### ONVIFが応答しない

1. 複数ポートを試行（80, 2020, 8899）
2. 認証が必要な場合はWS-Security認証ヘッダーを追加
3. ファームウェアでONVIFが無効化されている可能性を確認

### 並列接続が不安定

1. タイムアウト値を延長（5秒 → 10秒）
2. TCPトランスポートを明示指定（-rtsp_transport tcp）
3. ネットワーク帯域を確認
