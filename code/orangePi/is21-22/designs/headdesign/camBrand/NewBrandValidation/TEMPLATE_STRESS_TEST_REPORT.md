# {BRAND_NAME} {MODEL_NAME} ストレステスト報告書

## テスト実施日
YYYY-MM-DD

## テスト環境

| 項目 | 値 |
|------|-----|
| カメラ | {BRAND_NAME} {MODEL_NAME} |
| カメラIP | 192.168.XXX.XXX |
| テストサーバー | IS22 (192.168.125.246) |
| ONVIFポート | {ONVIF_PORT} |
| RTSPポート | 554 |
| 認証情報 | {USER}:{PASS} |
| 解像度(Main) | {MAIN_RESOLUTION} |
| 解像度(Sub) | {SUB_RESOLUTION} |
| RTSPパス | {MAIN_PATH}, {SUB_PATH} |

---

## 1. テスト結果サマリー

| テスト項目 | 結果 | 評価 |
|-----------|------|------|
| RTSP順次接続 | /5成功（%） | {評価} |
| RTSP急速再接続（インターバルなし） | /20成功（%） | {評価} |
| RTSP再接続（0.5秒間隔） | /10成功（%） | {評価} |
| RTSP再接続（1秒間隔） | /10成功（%） | {評価} |
| RTSP再接続（2秒間隔） | /10成功（%） | {評価} |
| RTSP長時間ストリーム（30秒） | {結果} | {評価} |
| RTSP 2並列接続 | /6成功（%） | {評価} |
| RTSP 3並列接続 | /9成功（%） | {評価} |
| RTSP 5並列接続 | /15成功（%） | {評価} |
| Main + Sub同時 | /3成功（%） | {評価} |
| ONVIF急速リクエスト | /20成功（%） | {評価} |
| HTTP急速リクエスト | /20成功（%） | {評価} |

---

## 2. 詳細テスト結果

### 2.1 RTSP順次接続テスト

| 接続 | 結果 | 応答時間 | 解像度 |
|------|------|---------|--------|
| 1 | {結果} | {time}ms | {解像度} |
| 2 | {結果} | {time}ms | {解像度} |
| 3 | {結果} | {time}ms | {解像度} |
| 4 | {結果} | {time}ms | {解像度} |
| 5 | {結果} | {time}ms | {解像度} |

**平均応答時間**: 約{avg}秒

---

### 2.2 RTSP再接続テスト

| インターバル | 成功率 | 詳細 |
|-------------|--------|------|
| 0秒（急速） | **%** (/20) | {評価} |
| 0.5秒 | **%** (/10) | {評価} |
| 1秒 | **%** (/10) | {評価} |
| 2秒 | **%** (/10) | {評価} |

**所見**: {所見を記載}

---

### 2.3 長時間ストリームテスト

```
結果: {SUCCESS/FAILED}
フレーム数: {N}フレーム
平均FPS: 約{N}fps
実行時間: {N}秒
警告/エラー: {あれば記載}
```

---

### 2.4 RTSP並列接続テスト

| 並列数 | 試行回数 | 成功率 | 詳細 |
|--------|---------|--------|------|
| 2 | 3回 | **%** (/6) | {評価} |
| 3 | 3回 | **%** (/9) | {評価} |
| 5 | 3回 | **%** (/15) | {評価} |

**所見**: {所見を記載}

---

### 2.5 Main + Sub 同時接続テスト

| 試行 | Main結果 | Sub結果 | 評価 |
|------|---------|--------|------|
| 1 | {結果} | {結果} | {評価} |
| 2 | {結果} | {結果} | {評価} |
| 3 | {結果} | {結果} | {評価} |

**所見**: {所見を記載}

---

### 2.6 ONVIFエンドポイントテスト

| テスト | 結果 | 備考 |
|-------|------|------|
| 急速リクエスト（20回） | /20 | {評価} |

---

### 2.7 HTTPエンドポイントテスト

| テスト | 結果 |
|-------|------|
| 急速リクエスト（20回） | /20 |

---

## 3. 性能特性

### 強み ✅
- {強み1}
- {強み2}
- {強み3}

### 弱み ⚠️
- {弱み1}
- {弱み2}
- {弱み3}

---

## 4. IS22運用への推奨事項

### 4.1 Access Absorber設定

```
推奨設定:
- family: {family_name}
- max_concurrent_streams: {N}
- min_reconnect_interval_ms: {N}
- require_exclusive_lock: {TRUE/FALSE}
```

### 4.2 スナップショット取得

```
推奨設定:
- Main/Sub同時取得: {可能/不可}
- スナップショット間隔: {N}秒以上
- リトライ回数: {N}回
```

### 4.3 go2rtcプロキシ

```
推奨: {必要/不要}
理由: {理由を記載}
```

---

## 5. 結論

**{BRAND_NAME} {MODEL_NAME}は{総評}。**

### 評価
| 項目 | スコア |
|------|--------|
| 再接続安定性 | ★☆☆☆☆ (/5) |
| 並列接続 | ★☆☆☆☆ (/5) |
| 長時間安定性 | ★☆☆☆☆ (/5) |
| Main/Sub同時 | ★☆☆☆☆ (/5) |
| **総合評価** | **★☆☆☆☆ ({N}/5)** |

---

## 6. DB投入SQL

```sql
-- camera_access_limits
INSERT INTO camera_access_limits
(family, display_name, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, notes)
VALUES
('{family}', '{BRAND_NAME}', {max_concurrent}, {min_interval}, {exclusive_lock}, 'Stress test YYYY-MM-DD');

-- rtsp_templates
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, '{BRAND_NAME} Standard', '{main_path}', '{sub_path}', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = '{family}';

-- oui_entries (必要な場合)
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT '{OUI_PREFIX}', b.id, '{OUI_VENDOR} - {DESCRIPTION}', 20, 'confirmed', '実機検証 YYYY-MM-DD', TRUE
FROM camera_brands b WHERE b.name = '{family}';
```

---

## 付録: テストコマンド

### RTSP接続テスト
```bash
ffprobe -rtsp_transport tcp -v error -show_entries stream=width,height -of csv=p=0 \
    "rtsp://{USER}:{PASS}@{CAMERA_IP}:554{MAIN_PATH}"
```

### 長時間ストリームテスト
```bash
timeout 35 ffmpeg -rtsp_transport tcp -i "rtsp://{USER}:{PASS}@{CAMERA_IP}:554{MAIN_PATH}" \
    -t 30 -f null -
```

### ONVIFテスト
```bash
curl -s --max-time 3 -X POST \
    -H "Content-Type: application/soap+xml" \
    -d '<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>' \
    http://{CAMERA_IP}:{ONVIF_PORT}/onvif/device_service
```
