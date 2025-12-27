# AraneaSDK mobes側機能要求書

**作成日**: 2025-12-26
**作成者**: aranea_ISMS開発ライン
**対象**: mobes2.0開発チーム

---

## 1. 現状の実装状況サマリー

### 1.1 ESP32デバイス実装状況

| デバイス | 状態報告 | コマンド受信 | MQTT | 備考 |
|---------|---------|------------|------|------|
| is04a | HTTP POST | HTTP API | 定義のみ未使用 | パルス制御 |
| is05a | HTTP POST | HTTP API + Webhook | 定義のみ未使用 | 8ch入力、ルール機能 |
| is06a | HTTP POST | HTTP API | 定義のみ未使用 | PWM制御 |
| is10 | HTTP POST (Celestial) | **MQTT config** | ✅使用中 | SSHポーラー |
| is10t | HTTP POST (Celestial) | 未実装 | 未使用 | ONVIFカメラ監視 |
| is01a | 未実装 | - | - | BLEセンサー（設計のみ） |
| is02a | 未実装 | - | - | BLEゲートウェイ（設計のみ） |

### 1.2 Orange Pi実装状況

| デバイス | 主要機能 | データ送信 | 大量データ |
|---------|---------|----------|----------|
| is03a | BLE受信リレー | HTTP POST | Forwarderキュー5000件 |
| is20s | パケットキャプチャ | **バッチ送信** | 500件/30秒、gzip、20000件キュー |

---

## 2. 機能要求一覧

### カテゴリA: 大量データ送信系

#### A-1. バッチ状態レポート受信エンドポイント

**背景**: is20sは30秒間隔で最大500件のパケットキャプチャイベントをバッチ送信する

**要求**:
```
POST /deviceStateReportBatch
Content-Type: application/x-ndjson
Content-Encoding: gzip

{...event1}\n
{...event2}\n
...
```

**仕様**:
- NDJSON（改行区切りJSON）対応
- gzip圧縮対応（必須）
- 1リクエストあたり最大500件
- レスポンス: 各イベントの処理結果（成功/失敗/重複）

**現状確認**: `deviceStateReportBatch`は存在するか？is20s用の大量データ受信に対応しているか？

#### A-2. is20s専用: パケットキャプチャ受信

**背景**: is20sはER605ミラーポートからパケットをキャプチャし、以下の情報を送信

**送信データ構造**:
```json
{
  "id": "10120251225143005-0834",
  "epoch": 1735106405.123,
  "time": "2025-12-25T14:30:05+09:00",
  "room_no": "101",
  "src_ip": "192.168.3.101",
  "dst_ip": "142.250.196.110",
  "dst_port": "443",
  "protocol": "tcp",
  "dns_qry": null,
  "tls_sni": "www.youtube.com",
  "asn": 15169,
  "asn_service": "YouTube/Google",
  "asn_category": "video",
  "threat": null
}
```

**要求**:
1. is20s専用のバッチ受信エンドポイント
2. BigQuery連携（パケットログ長期保存）
3. 脅威検知イベントのアラート機能

---

### カテゴリB: BLE MAC補完・LacisID構成系

#### B-1. BLEアドバタイズからのLacisID再構成

**背景**: is01a（BLEセンサー）は自身のMACをアドバタイズに含めて送信。is02a/is03aがこれを受信してLacisIDを再構成する。

**フロー**:
```
is01a (BLE Advertise)
  ↓ MAC: AABBCCDDEEFF
is02a/is03a (受信・再構成)
  ↓ LacisID: 3001AABBCCDDEEFF0001
mobes (登録・状態管理)
```

**要求**:
1. **MAC→LacisID再構成API**（確認用）
   ```
   POST /api/lacisId/reconstruct
   {
     "productType": "001",
     "mac": "AABBCCDDEEFF",
     "productCode": "0001"
   }
   → { "lacisId": "3001AABBCCDDEEFF0001" }
   ```

2. **ゲートウェイ経由登録**
   - is02a/is03aがis01aを代理登録するフロー
   - 親デバイス（ゲートウェイ）のCICで子デバイス（センサー）を登録

#### B-2. BLEペイロード解析サポート

**is01a BLEペイロード形式（ISMSv1）**:
```
[I][S][Version=0x01][ProductType][Flags][MAC(6B)][BootCount(4B)][Temp(f32)][Hum(f32)][Batt(1B)]
```

**要求**: mobes側でもペイロード解析できるようスキーマ定義を共有

---

### カテゴリC: Pub/Sub・リアルタイム制御系

#### C-1. MQTT Pub/Subテスト機能

**背景**: is10ではMQTT config受信が実装済み。is04a/is05a/is06aはMQTT定義のみで未使用。

**要求**:
1. **MQTTテスト送信ツール**（AdminSettings UIから）
   - 任意のトピックにテストメッセージ送信
   - デバイスの応答（ACK）確認

2. **トピック構造の明確化**
   ```
   # 現在のis10実装
   {fid}/devices/{cic}/config  # 設定プッシュ

   # 要求: 統一トピック体系
   aranea/{tid}/{lacisId}/command   # コマンド送信
   aranea/{tid}/{lacisId}/config    # 設定プッシュ
   aranea/{tid}/{lacisId}/state     # 状態報告（双方向デバイス）
   aranea/{tid}/{lacisId}/ack       # ACK応答
   ```

#### C-2. コマンド送信・ACK確認

**対応デバイス・コマンド一覧**:

| デバイス | コマンド | パラメータ | 備考 |
|---------|---------|----------|------|
| is04a | `pulse` | output(1-2), duration | パルス出力 |
| is04a | `set` | output(1-2), active(bool) | 持続出力 |
| is05a | `pulse` | channel(7-8), duration | ch7/ch8パルス |
| is05a | `setMode` | channel(1-8), mode(input/output) | I/Oモード切替 |
| is06a | `pulse` | output(1-6) | トリガー出力 |
| is06a | `setPwm` | output(1-4), duty(0-100) | PWMデューティー |
| is10 | `updateRouters` | routers[] | ルーター設定更新 |

**要求**:
1. **コマンド送信API**（HTTP経由でMQTTへ）
   ```
   POST /araneaDeviceCommand/{lacisId}
   {
     "command": "pulse",
     "params": { "output": 1, "duration": 3000 },
     "ttlSec": 60
   }
   ```

2. **ACK待機・タイムアウト処理**
   - ACK受信でコマンド完了
   - ttlSec経過でタイムアウト
   - 結果をレスポンスまたはWebhookで通知

#### C-3. PWM数値系送受信

**is06a PWM制御**:
```json
// 設定プッシュ
{
  "command": "setPwm",
  "params": {
    "output": 1,
    "duty": 75,
    "frequency": 1000
  }
}

// 状態報告
{
  "state": {
    "trg1": { "active": true, "pwmDuty": 75 },
    "trg2": { "active": false, "pwmDuty": 0 }
  }
}
```

**要求**: PWM値（0-100%）の送受信・表示対応

---

### カテゴリD: ステータス・設定更新系

#### D-1. デバイス設定プッシュ

**現状**: is10のみMQTT経由で設定受信実装済み

**要求**:
1. **設定プッシュAPI**
   ```
   POST /araneaDeviceConfig/{lacisId}
   {
     "config": {
       "pulseMs": 3000,
       "interlockMs": 200,
       "debounceMs": 50
     },
     "merge": true
   }
   ```

2. **設定バージョン管理**
   - デバイス側で設定バージョン保持
   - 古いバージョンの設定は無視

#### D-2. デバイス状態取得

**要求**:
```
GET /araneaDeviceGetState/{lacisId}

Response:
{
  "ok": true,
  "device": {
    "lacisId": "3004AABBCCDDEEFF0001",
    "type": "aranea_ar-is04a",
    "state": {
      "output1": { "active": true },
      "output2": { "active": false }
    },
    "config": {
      "pulseMs": 3000
    },
    "alive": true,
    "lastUpdatedAt": "2025-12-26T00:00:00Z",
    "rssi": -65,
    "heap": 120000,
    "firmwareVersion": "1.0.0"
  }
}
```

#### D-3. 相互更新（双方向同期）

**要求**: デバイス設定変更時の通知機構
1. デバイスがWeb UIで設定変更 → mobesに通知
2. mobesが設定変更 → デバイスに通知（MQTT config）
3. 競合解決ポリシー（タイムスタンプベース）

---

### カテゴリE: IoT動作テスト機能

#### E-1. 正規経路テストフロー

**要求**: AdminSettings UIから以下のテストフローを実行可能に

```
[テスト開始]
    ↓
1. コマンド送信（mobes → MQTT → デバイス）
    ↓
2. ACK確認（デバイス → MQTT → mobes）
    ↓
3. 状態報告確認（デバイス → HTTP → mobes）
    ↓
4. BigQuery記録確認
    ↓
[結果表示]
```

**UIモックアップ**:
```
┌─────────────────────────────────────────┐
│ IoT動作テスト                            │
├─────────────────────────────────────────┤
│ デバイス: [3004AABBCCDDEEFF0001 ▼]      │
│ コマンド: [pulse ▼]                      │
│ パラメータ: output=1, duration=3000     │
│                                         │
│ [テスト実行]                            │
│                                         │
│ 結果:                                   │
│ ✅ コマンド送信: 成功 (123ms)           │
│ ✅ ACK受信: 成功 (456ms)                │
│ ✅ 状態報告: 成功 (789ms)               │
│ ✅ BigQuery記録: evt_1234567890         │
└─────────────────────────────────────────┘
```

#### E-2. デバイスシミュレーター

**要求**: 実機なしでのテスト用シミュレーター
- 仮想デバイス作成（LacisID指定）
- 状態報告の模擬送信
- コマンド受信の模擬応答

---

### カテゴリF: ログ・履歴取得系

#### F-1. 状態レポート履歴取得

**要求**:
```
GET /araneaDeviceLogs/{lacisId}?limit=100&offset=0&from=2025-12-25T00:00:00Z&to=2025-12-26T00:00:00Z

Response:
{
  "ok": true,
  "logs": [
    {
      "id": "evt_1234567890",
      "observedAt": "2025-12-25T12:00:00Z",
      "state": { ... },
      "meta": { "rssi": -65, "heap": 120000 }
    },
    ...
  ],
  "total": 1500,
  "hasMore": true
}
```

**要求パラメータ**:
- `limit`: 取得件数（デフォルト100、最大1000）
- `offset`: オフセット
- `from`, `to`: 期間指定（ISO8601）
- `stateFilter`: 状態フィルター（例: `output1.active=true`）

#### F-2. コマンド履歴取得

**要求**:
```
GET /araneaDeviceCommandLogs/{lacisId}?limit=50

Response:
{
  "ok": true,
  "logs": [
    {
      "commandId": "cmd_123456",
      "command": "pulse",
      "params": { "output": 1, "duration": 3000 },
      "sentAt": "2025-12-25T12:00:00Z",
      "status": "executed",
      "ackAt": "2025-12-25T12:00:01Z",
      "resultState": { "output1": { "active": true } }
    },
    ...
  ]
}
```

#### F-3. デバイスイベントストリーム（SSE/WebSocket）

**要求**: リアルタイム監視用

```
GET /araneaDeviceStream/{lacisId}
Accept: text/event-stream

event: state
data: {"output1":{"active":true},"observedAt":"..."}

event: command
data: {"commandId":"cmd_123","command":"pulse","status":"executed"}

event: config
data: {"pulseMs":3000,"updatedAt":"..."}
```

---

### カテゴリG: Webhook機能拡張

#### G-1. mobes側Webhook送信

**背景**: is05aはDiscord/Slack/Generic Webhookを送信可能。mobes側からも同様の通知が必要。

**要求**:
1. **アラート条件設定**
   - デバイスオフライン検知
   - 異常値検知（閾値超過）
   - 脅威検知（is20s）

2. **Webhook送信先設定**
   - Discord/Slack/Microsoft Teams/Generic
   - テナント/施設/デバイス単位で設定

#### G-2. is05a Webhook連携

**現在のis05a Webhookフォーマット**:
```json
// Discord
{ "content": "🚨 **窓1** が **開** になりました", "embeds": [...] }

// Slack
{ "text": "*窓1* が *開* になりました", "attachments": [...] }

// Generic
{
  "device_id": "3005...",
  "lacis_id": "3005...",
  "event": "input_change",
  "channel": 1,
  "state": true,
  "timestamp": "2025-12-25T12:00:00Z"
}
```

**要求**: mobes側でGeneric Webhookを受信・処理するエンドポイント

---

### カテゴリH: 脅威インテリジェンス連携（is20s）

#### H-1. 脅威検知イベント受信

**is20s脅威検知データソース**:
- StevenBlack/hosts (malware, fakenews, gambling, porn)
- dan.me.uk (Tor出口ノード)

**要求**:
1. 脅威検知イベントの受信・保存
2. アラート生成（Webhook/Push通知）
3. 脅威統計ダッシュボード

#### H-2. 脅威リスト同期

**要求**: mobes側で管理する脅威リストをis20sにプッシュ
- カスタムブロックリスト
- ホワイトリスト
- テナント固有のポリシー

---

## 3. 優先度マトリクス

| 要求ID | カテゴリ | 優先度 | 依存関係 |
|--------|---------|--------|---------|
| A-1 | バッチ受信 | **P0** | is20s稼働に必須 |
| C-1 | MQTTテスト | **P0** | 開発効率に必須 |
| C-2 | コマンド送信・ACK | **P1** | 双方向制御に必須 |
| D-1 | 設定プッシュ | **P1** | リモート管理に必須 |
| D-2 | 状態取得 | **P1** | 監視UIに必須 |
| F-1 | ログ取得 | **P1** | デバッグに必須 |
| E-1 | IoTテストフロー | **P2** | 開発効率向上 |
| B-1 | MAC補完API | **P2** | is01a/is02a実装時に必要 |
| F-3 | イベントストリーム | **P2** | リアルタイム監視 |
| G-1 | mobes Webhook | **P3** | アラート機能 |
| H-1 | 脅威検知 | **P3** | セキュリティ機能 |
| E-2 | シミュレーター | **P3** | テスト効率化 |

---

## 4. テスト用認証情報

| 項目 | 値 |
|------|-----|
| TID | `T9999999999999999999` |
| LacisID | `17347487748391988274` |
| CIC | `022029` |
| Email | `dev@araneadevice.dev` |

---

## 5. 回答依頼

以下の形式で回答をお願いします：

```
### A-1. バッチ状態レポート受信
- 対応状況: [実装済み / 開発中 / 未着手 / 対応予定なし]
- 対応予定: [時期またはPhase]
- 備考:

### C-1. MQTTテスト機能
- 対応状況: [...]
...
```

---

## 6. 連絡先

- aranea_ISMS開発: Claude Code (このドキュメント)
- 確認先: mobes2.0開発チーム
