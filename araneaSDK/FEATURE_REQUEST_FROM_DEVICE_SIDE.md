# AraneaSDK 機能要求 - デバイス開発者視点

**作成日**: 2025-12-26
**作成者**: aranea_ISMS開発チーム
**対象**: mobes2.0 AraneaSDK開発チーム

---

## 現状評価

SDK CLI v0.1.4 は基本機能が動作するようになりましたが、**実際のESP32開発で必要なテスト機能が不足**しています。

---

## 0. Gate API stateEndpoint返却 (P0) - 最優先

### 問題

現在 `araneaDeviceGate` の登録レスポンスに `stateEndpoint` が含まれていません。

デバイスは登録後にCICを取得できますが、**状態レポートの送信先URLが不明**なため、deviceStateReportを送信できません。

### 現状のレスポンス

```json
{
  "ok": true,
  "userObject": {
    "cic_code": "xxxxxx"
  }
}
```

### 必要なレスポンス

```json
{
  "ok": true,
  "userObject": {
    "cic_code": "xxxxxx"
  },
  "stateEndpoint": "https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport",
  "mqttEndpoint": "wss://asia-northeast1-mobesorder.cloudfunctions.net/araneaMqtt"
}
```

### 影響

- **is10**: 登録済みだがstateEndpointが空のため401エラー
- **全デバイス**: 新規登録時にstateEndpointが取得できない

### 暫定対応（デバイス側）

デバイス側では以下のフォールバックを実装予定：
```cpp
String stateEndpoint = araneaReg.getSavedStateEndpoint();
if (stateEndpoint.length() == 0) {
  // Gate APIが返さない場合はデフォルト使用
  stateEndpoint = ARANEA_DEFAULT_CLOUD_URL;
}
```

ただし、将来的なエンドポイント変更に対応するため、**Gate APIでの返却が必須**です。

---

## 1. MQTT テスト機能 (P0)

### 1.1 MQTTダミーコマンド送信

ESP32デバイスは以下のトピックを購読しています：
```
aranea/{tid}/{lacisId}/command
aranea/{tid}/{lacisId}/config
```

**必要な機能**:
```bash
# コマンド送信テスト
aranea-sdk mqtt send-command \
  --tid T9999999999999999999 \
  --lacis-id 3004AABBCCDDEEFF0001 \
  --command '{"pulse":{"output":1,"durationMs":3000}}'

# 設定変更送信テスト
aranea-sdk mqtt send-config \
  --tid T9999999999999999999 \
  --lacis-id 3004AABBCCDDEEFF0001 \
  --config '{"pulseMs":5000,"interlockMs":300}'
```

### 1.2 MQTT購読モニター

デバイスからのACK/echoを監視：
```bash
# ACKモニター
aranea-sdk mqtt monitor \
  --tid T9999999999999999999 \
  --lacis-id 3004AABBCCDDEEFF0001 \
  --topics ack,echo \
  --timeout 60
```

### 1.3 定期送信モード

開発中のESP32に定期的にコマンドを送信：
```bash
# 10秒間隔でpulseコマンドを送信（デバイス動作テスト用）
aranea-sdk mqtt periodic \
  --tid T9999999999999999999 \
  --lacis-id 3004AABBCCDDEEFF0001 \
  --command '{"pulse":{"output":1}}' \
  --interval 10 \
  --count 5
```

---

## 2. レスポンス遅延テスト (P1)

### 2.1 スローレスポンスシミュレーション

ESP32の実装で見落としがちな問題：
- API応答が遅い場合のタイムアウト処理
- 再試行ロジック
- Watchdog対策

**必要な機能**:
```bash
# 遅延シミュレーション付きstate report
aranea-sdk test latency \
  --type aranea_ar-is04a \
  --delay 5000 \
  --tid T9999999999999999999

# タイムアウト閾値テスト
aranea-sdk test timeout \
  --endpoint state \
  --timeout-ms 3000,5000,10000,30000
```

### 2.2 ネットワーク品質シミュレーション

```bash
# パケットロスシミュレーション
aranea-sdk test network \
  --packet-loss 10 \
  --latency 500 \
  --jitter 200
```

---

## 3. エラーケーステスト (P1)

### 3.1 認証エラーシミュレーション

```bash
# 無効なCICでテスト
aranea-sdk test auth-error \
  --tid T9999999999999999999 \
  --lacis-id 3004AABBCCDDEEFF0001 \
  --cic INVALID

# 期待されるエラーコード確認
# → AR-1002: CIC_MISMATCH
```

### 3.2 スキーマ不一致テスト

```bash
# 間違ったフィールドでstate reportを送信
aranea-sdk test schema-error \
  --type aranea_ar-is04a \
  --state '{"wrongField":123}'

# → AR-2003: INVALID_STATE_FIELD
```

### 3.3 レート制限テスト

```bash
# 連続送信でレート制限を確認
aranea-sdk test rate-limit \
  --type aranea_ar-is04a \
  --burst 100 \
  --interval 100

# → AR-4001: RATE_LIMIT_EXCEEDED (何回目で発生するか)
```

---

## 4. E2Eインテグレーションテスト (P2)

### 4.1 デバイスライフサイクルテスト

```bash
# 登録 → 状態報告 → コマンド受信 → ACK の一連フロー
aranea-sdk test e2e \
  --type aranea_ar-is04a \
  --mac AABBCCDDEEFF \
  --scenario lifecycle

# 出力:
# 1. [OK] デバイス登録
# 2. [OK] CIC取得
# 3. [OK] 状態レポート送信
# 4. [WAIT] MQTT接続...
# 5. [OK] コマンド受信
# 6. [OK] ACK送信
# 総合結果: PASS
```

### 4.2 再接続テスト

```bash
# WiFi切断→再接続シナリオ
aranea-sdk test reconnect \
  --disconnect-after 10 \
  --reconnect-delay 5 \
  --verify-state-sync
```

---

## 5. 負荷テスト (P2)

### 5.1 バースト送信

```bash
# 短時間に大量のstate reportを送信
aranea-sdk test burst \
  --type aranea_ar-is04a \
  --count 100 \
  --interval 100 \
  --measure-latency

# 結果:
# 成功: 98/100
# 失敗: 2/100 (rate limit)
# 平均レイテンシ: 245ms
# P95レイテンシ: 890ms
# P99レイテンシ: 1250ms
```

### 5.2 長時間安定性テスト

```bash
# 24時間連続動作テスト
aranea-sdk test stability \
  --duration 24h \
  --report-interval 5m \
  --output stability_report.json
```

---

## 6. デバッグ支援機能 (P2)

### 6.1 リクエスト/レスポンスログ

```bash
# 詳細ログモード
aranea-sdk --verbose simulate state-report \
  --type aranea_ar-is04a \
  --dry-run

# 出力:
# [DEBUG] Request URL: https://...
# [DEBUG] Request Headers: {...}
# [DEBUG] Request Body: {...}
# [DEBUG] Response Status: 200
# [DEBUG] Response Time: 234ms
# [DEBUG] Response Body: {...}
```

### 6.2 cURLエクスポート

```bash
# ESP32のHTTPClientで使えるcURLコマンドを生成
aranea-sdk export curl \
  --type aranea_ar-is04a \
  --action state-report

# 出力:
# curl -X POST https://asia-northeast1-mobesorder.cloudfunctions.net/deviceStateReport \
#   -H "Content-Type: application/json" \
#   -d '{"auth":{...},"report":{...}}'
```

### 6.3 Arduino/ESP32コード生成

```bash
# ESP32用のサンプルコード生成
aranea-sdk generate arduino \
  --type aranea_ar-is04a \
  --output is04a_sample.ino

# 出力: StateReporter, MQTTHandler, WiFiManager のサンプルコード
```

---

## 7. ローカルモックサーバー (P3)

### 7.1 オフライン開発用

```bash
# ローカルにモックサーバーを起動
aranea-sdk mock start \
  --port 8080 \
  --schema aranea_ar-is04a

# ESP32から http://192.168.x.x:8080/deviceStateReport に接続
# インターネット不要でAPI動作をテスト
```

### 7.2 カスタムレスポンス設定

```bash
# エラーレスポンスを返すモック
aranea-sdk mock start \
  --response-file custom_responses.json \
  --error-rate 10
```

---

## 優先度まとめ

| 優先度 | 機能 | 理由 |
|--------|------|------|
| **P0** | Gate API stateEndpoint返却 | **デバイスがstateReport送信不可** |
| **P0** | MQTTテスト | コマンド受信のテストが現状不可能 |
| **P1** | 遅延テスト | タイムアウト実装の検証に必須 |
| **P1** | エラーケース | 異常系の動作確認に必須 |
| **P2** | E2Eテスト | 統合テストの自動化 |
| **P2** | 負荷テスト | 実運用前の品質確認 |
| **P2** | デバッグ支援 | 開発効率向上 |
| **P3** | モックサーバー | オフライン開発 |

---

## 期待される開発ワークフロー

```
1. aranea-sdk schema get --type aranea_ar-is04a
   → スキーマ確認

2. aranea-sdk generate arduino --type aranea_ar-is04a
   → サンプルコード生成

3. aranea-sdk register --type aranea_ar-is04a --mac AABBCCDDEEFF
   → デバイス登録

4. aranea-sdk mqtt monitor --lacis-id 3004...
   → MQTTモニター開始

5. aranea-sdk mqtt send-command --command '{"pulse":{"output":1}}'
   → コマンド送信テスト

6. aranea-sdk test e2e --scenario lifecycle
   → 統合テスト実行
```

---

## 連絡先

aranea_ISMS開発チーム

**これらの機能があれば、ESP32開発の効率と品質が大幅に向上します。**
