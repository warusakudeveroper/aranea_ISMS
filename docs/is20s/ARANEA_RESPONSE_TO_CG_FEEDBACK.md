# IS-20S統合設計 CelestialGlobe回答書への返答

## メタデータ

| 項目 | 値 |
|-----|---|
| From | Aranea Device開発実装チーム |
| To | CelestialGlobe実装班 |
| Date | 2025-12-29 |
| Subject | RESPONSE_TO_ARANEA_FEEDBACK.md への返答 |
| Reference | コミット a21e59ff |

---

## 1. 回答書受領確認

CelestialGlobe実装班からの回答書を受領しました。
責務境界に関する認識が概ね一致しており、建設的な議論が可能と考えます。

---

## 2. 責務境界案への同意

### 2.1 CelestialGlobe責務（同意）

| 機能 | Aranea側見解 |
|-----|-------------|
| nodeOrderステータス表示 | ✅ 同意 |
| トラフィックデータ受信 | ✅ 同意 |
| BigQuery書き込み | ✅ 同意 |
| domainDict読み取り（表示用） | ✅ 同意 |
| トラフィック統計UI | ✅ 同意 |

### 2.2 AraneaDeviceGate責務（同意）

| 機能 | Aranea側見解 |
|-----|-------------|
| クラウド→デバイス設定変更 | ✅ 同意（Gate責務が妥当） |
| 辞書（domain_services）同期 | ✅ 同意（Gate責務が妥当） |
| トラフィック絞り設定 | ✅ 同意（Gate責務が妥当） |
| デバイス認証・Provisioning | ✅ 同意（Gate責務が妥当） |

### 2.3 共同責務項目への意見

| 機能 | CG案 | Aranea側提案 |
|-----|------|-------------|
| IP→部屋番号マッピング | CG:保存、Gate:配布 | ✅ 同意。ただしUI編集はCG側で提供希望 |
| unknownDomains処理 | CG受信→Metatron分析 | ✅ 同意。辞書更新はGate経由でデバイスに配布 |
| domainDict更新 | 未定 | **提案**: マスターはFirestore、更新権限はGate |

---

## 3. 技術的確認事項への回答

### Q1: X-Aranea-Mac

**回答**: デバイス側で自己申告（ESP32のSTA MAC）

```cpp
// ESP32側実装
WiFi.macAddress()  // → "AA:BB:CC:DD:EE:FF"
// lacisID生成時に使用済み
```

MACの偽装リスクは認識していますが、lacisIdとCICの組み合わせで認証するため許容可能と考えます。

### Q2: CIC検証

**回答**: AraneaDeviceGate側で実装済み

- CIC検証ロジックは `araneaDeviceGate_auth` Cloud Functionで実装済み
- IS-20Sも同様のフローで認証可能

### Q3: summaryフィールド解決

**提案**: デバイス側で解決

理由:
- デバイス側にdomain_services辞書（現在445エントリ）を保持済み
- リアルタイムでservice_id, category, threat_levelを付与可能
- クラウド側の処理負荷軽減

```typescript
// デバイス側で付与するsummary
summary: {
  service_id: "netflix",
  service_category: "streaming",
  threat_level: 0  // 0=safe, 1=caution, 2=threat
}
```

**unknownドメインの場合**:
```typescript
summary: {
  service_id: null,
  service_category: "unknown",
  threat_level: 1  // 未知は要注意扱い
}
```

### Q4: 辞書同期頻度

**提案**: 定期バッチ（24時間間隔）+ イベント駆動

```
定期同期: 24時間ごとにGateから最新辞書をpull
イベント駆動: Gateから「辞書更新通知」受信時に即時pull
```

### Q5: 設定変更通知

**提案**: MQTTを推奨

理由:
- IS-20SはOrange Pi Zero3で動作、常時接続可能
- 既存ESP32デバイスもMQTT対応済み
- 双方向通信が容易

```
Topic: aranea/{tid}/is20s/{lacisId}/config
Payload: { "action": "update", "configVersion": 123 }
```

WebSocketも可能だが、MQTTの方がIoTデバイス向けに最適化されている。

---

## 4. sourceType命名規則への同意

**同意**: `ar-is20s` で確定

```
X-Aranea-SourceType: ar-is20s
```

既存デバイスとの整合性:
- `aranea_ar-is01` → スキーマレジストリ用（型定義）
- `ar-is20s` → ヘッダー用（ソース識別）

両者は用途が異なるため、命名規則の差異は許容可能。

---

## 5. TrafficPacketスキーマへの同意・補足

**基本同意**。以下を補足:

```typescript
interface Is20sTrafficPacket {
  // 必須フィールド
  ts: string;           // ISO8601 (例: "2025-12-29T10:30:00.123Z")
  src_ip: string;       // "192.168.3.101"
  dst_ip: string;       // "142.250.196.110"
  port: number;         // 443
  protocol: 'TCP' | 'UDP';
  room_no: string;      // "301" or "UNK"

  // オプション（デバイス側で解決済みの場合）
  domain?: string;              // "www.google.com"
  bytes?: number;               // 1500
  direction?: 'outbound';       // IS-20Sは基本outboundのみ

  // デバイス側で解決するsummary
  summary?: {
    service_id: string | null;      // "google" or null
    service_category: string;       // "search" or "unknown"
    threat_level: 0 | 1 | 2;        // 0=safe, 1=caution, 2=threat
  };
}
```

**補足**:
- `direction`: IS-20Sはルーター通過トラフィックを監視するため、基本的にoutbound
- `room_no: "UNK"`: IPマッピングがない場合のデフォルト値

---

## 6. IP→部屋番号マッピング運用案への同意

**同意**。以下の運用を確認:

```
1. マスターデータ: Firestore (facilities/{fid}/ipRoomMapping)
2. 編集UI: CelestialGlobe側で提供
3. デバイスへの配布: AraneaDeviceGate経由
4. デバイス側: ローカルキャッシュとして保持
5. 更新通知: MQTT経由でGateから通知 → デバイスがpull
```

**デバイス側キャッシュ形式**:
```json
{
  "version": 5,
  "updatedAt": "2025-12-29T10:00:00Z",
  "mappings": {
    "192.168.3.100-109": "301",
    "192.168.3.110-119": "302"
  }
}
```

---

## 7. 段階的実装アプローチへの同意

**Phase A/B アプローチに同意**

### Phase A（先行実装可能）

| 項目 | CelestialGlobe | AraneaDevice |
|-----|---------------|--------------|
| IS-20SのnodeOrder表示 | ✅ 実装 | - |
| NDJSON ingest受信 | ✅ 実装 | ✅ 送信実装 |
| BigQuery書き込み | ✅ 実装 | - |
| 基本トラフィック表示 | ✅ 実装 | - |

### Phase B（責務確定後）

| 項目 | 責務 |
|-----|------|
| 設定管理UI/API | Gate |
| 辞書同期 | Gate |
| トラフィック絞り | Gate |
| 詳細分析UI | CelestialGlobe |

---

## 8. 追加確認事項

### 8.1 ingestエンドポイント詳細

以下の仕様で実装準備可能か確認:

```http
POST /celestialGlobe_ingest?fid={fid}&source=araneaDevice
Content-Type: application/x-ndjson
Content-Encoding: gzip
X-Aranea-SourceType: ar-is20s
X-Aranea-LacisId: 300...
X-Aranea-Cic: 123456
X-Aranea-Mac: AA:BB:CC:DD:EE:FF

{...packet1...}
{...packet2...}
```

### 8.2 エラーレスポンス

以下の形式を想定:

| ステータス | 意味 | デバイス側対応 |
|-----------|------|---------------|
| 200 | 成功 | 正常終了 |
| 401 | 認証失敗（CIC無効等） | 再認証フロー |
| 429 | レート制限 | バックオフ後リトライ |
| 500 | サーバーエラー | リトライ |

### 8.3 バッチサイズ制限

推奨値を確認:
- 1バッチあたりの最大パケット数: 10,000?
- 1バッチあたりの最大サイズ: 1MB (gzip後)?
- ingest間隔: 5分?

---

## 9. 次のアクション

| # | アクション | 担当 | 備考 |
|---|----------|------|------|
| 1 | 本返答書の確認 | CelestialGlobe実装班 | - |
| 2 | AraneaDeviceGateチームへの協議依頼 | 両チーム | 責務境界確定 |
| 3 | Phase A仕様確定 | 両チーム | 上記確認事項含む |
| 4 | Phase A実装着手 | 両チーム | 仕様確定後 |

---

## 10. 連絡先

- **リポジトリ**: warusakudeveroper/aranea_ISMS
- **ドキュメントパス**: docs/is20s/

---

**Aranea Device開発実装チーム**
