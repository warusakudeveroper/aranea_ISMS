# IS-20S統合設計 修正要望書

**From**: Aranea Device開発実装チーム
**To**: CelestialGlobe実装班
**Date**: 2025-12-29
**Subject**: REVIEW_REQUEST_IS10_IS20S_INTEGRATION.md に対するフィードバック

---

## 1. 総合評価

設計書の方向性は妥当ですが、**現状の粒度では実装に着手できません**。
特に以下の点について詳細化・明確化が必要です。

---

## 2. 責務境界の確認要求（最重要）

### 2.1 確認が必要な事項

以下の機能について、**CelestialGlobe と AraneaDeviceGate のどちらが責務を持つか**を明確にしてください。

| 機能 | 想定される責務 | 確認要求 |
|------|---------------|----------|
| IS-20S設定変更（リモート） | AraneaDeviceGate? | 要確認 |
| 辞書（domain_services）同期 | AraneaDeviceGate? | 要確認 |
| トラフィック絞り設定 | AraneaDeviceGate? | 要確認 |
| IP→部屋番号マッピング管理 | AraneaDeviceGate? | 要確認 |
| ingestデータ受信・保存 | CelestialGlobe | 確定 |
| トラフィック可視化UI | CelestialGlobe | 確定 |

### 2.2 理由

IS-20Sの設定管理・辞書同期・フィルタリング設定は**IoTデバイス管理の範疇**であり、AraneaDeviceGate側の機能である可能性が高いです。

CelestialGlobe実装班がこれらを実装する場合、AraneaDeviceGate側との重複・整合性問題が発生します。

**実装着手前に必ずAraneaDeviceGateチームと責務境界を確定してください。**

---

## 3. 設計詳細化要求

### 3.1 クラウド→デバイス設定変更機能

現行設計書ではデバイス側ローカル管理が前提ですが、**クラウド側からの遠隔設定が必須要件**です。

#### 必要な設定項目

```yaml
is20s_config:
  # 送信設定
  ingest:
    enabled: true
    interval_sec: 300        # ingest間隔
    max_packets_per_batch: 10000
    compression: gzip

  # トラフィック絞り設定
  filter:
    enabled: true
    exclude_categories:      # 除外カテゴリ
      - "ad"
      - "tracker"
      - "analytics"
    include_only_rooms: []   # 空=全部屋、指定時=指定部屋のみ
    min_packet_size: 0

  # 辞書設定
  dictionary:
    auto_sync: true
    sync_interval_sec: 86400  # 24時間

  # IP→部屋番号マッピング
  room_mapping:
    "192.168.3.100-109": "301"
    "192.168.3.110-119": "302"
```

#### 要求仕様

1. **設定取得API**（デバイス→クラウド）
   - デバイス起動時・定期的に設定をpull
   - エンドポイント・認証方式の明確化

2. **設定プッシュ**（クラウド→デバイス）
   - MQTTコマンド or ポーリングベース
   - 方式の決定と仕様明確化

3. **設定変更UI**（CelestialGlobe側）
   - 施設管理画面からのIS-20S設定変更
   - または AraneaDeviceGate側のデバイス管理画面

---

### 3.2 辞書（domain_services）同期機能

IS-20Sは各デバイスで `domain_services.json` を保持しています。

#### 現状の課題

- 複数IS-20Sデバイス間で辞書が分散
- 手動同期が必要
- 新規ドメイン発見時の共有手段がない

#### 要求仕様

1. **マスター辞書管理**
   - クラウド側でマスター辞書を管理
   - 責務: CelestialGlobe or AraneaDeviceGate？

2. **同期API**
   ```
   GET  /api/is20s/dictionary          # マスター辞書取得
   POST /api/is20s/dictionary/merge    # デバイス側の新規発見をマージ
   ```

3. **Unknown Domains報告**
   - デバイスが検出した未知ドメインをクラウドに報告
   - クラウド側で分類・マスター辞書に追加
   - 全デバイスに配信

---

### 3.3 トラフィック絞り設定

#### 要求仕様

1. **カテゴリベースフィルタ**
   - ad, tracker, analytics等を送信対象から除外
   - クラウド側から除外カテゴリを設定可能

2. **部屋ベースフィルタ**
   - 特定部屋のみ監視対象にする
   - クラウド側から対象部屋を設定可能

3. **サービスベースフィルタ**
   - 特定サービス（例: Netflix, YouTube）のみ送信
   - または特定サービスを除外

---

### 3.4 IP→部屋番号マッピング

#### 現行設計の問題

設計書では「デバイス側管理」となっていますが：

- 複数デバイスで同一マッピングが必要
- 施設のIP割り当て変更時に全デバイス更新が必要
- **クラウド側で一元管理すべき**

#### 要求仕様

1. **マッピング管理API**
   ```
   GET  /api/facility/{fid}/room-mapping
   PUT  /api/facility/{fid}/room-mapping
   ```

2. **デバイスへの配信**
   - マッピング変更時にデバイスへプッシュ
   - またはデバイスが定期的にpull

---

## 4. ペイロードフォーマット確認

### 4.1 sourceType命名規則

設計書: `"sourceType": "ar-is20s"`

既存デバイスは `aranea_ar-is01`, `aranea_ar-is04a` 等のプレフィックス付き命名。

**確認要求**:
- `ar-is20s` で確定か？
- `aranea_ar-is20s` に統一すべきか？
- スキーマレジストリとの整合性

### 4.2 packets配列の詳細仕様

以下の項目を明確化してください：

```typescript
interface TrafficPacket {
  timestamp: string;      // ISO8601? Unix epoch?
  srcIp: string;
  dstIp?: string;
  domain: string;
  protocol: "dns" | "http" | "https" | "quic";
  category?: string;      // サービスカテゴリ
  serviceName?: string;   // サービス名
  roomNo?: string;        // 部屋番号
  bytesSent?: number;
  bytesReceived?: number;
  // 他に必要なフィールドは？
}
```

### 4.3 summary の詳細仕様

```typescript
interface TrafficSummary {
  periodStart: string;
  periodEnd: string;
  totalPackets: number;
  // 他に何が必要か？
  byCategory?: Record<string, number>;
  byRoom?: Record<string, number>;
  topDomains?: Array<{domain: string, count: number}>;
}
```

---

## 5. 実装タスクへのフィードバック

### M8-IS12: 部屋番号マッピングUI

**不足**:
- UIのワイヤーフレーム
- APIエンドポイント仕様
- 責務（CelestialGlobe側 or AraneaDeviceGate側）

### M8-IS14: デバイス設定同期

**不足**:
- 同期プロトコル（MQTT? HTTP polling?）
- 設定スキーマ
- 競合解決ルール

---

## 6. 質問事項

1. **ingestエンドポイントの認証方式**
   - ヘッダー形式: `X-Lacis-Id`, `X-Cic` で確定か？
   - または `Authorization: Bearer {token}` 形式か？

2. **エラーレスポンス仕様**
   - 認証失敗時: 401? 403?
   - レート制限時: 429?
   - デバイス側のリトライ戦略

3. **データ保持期間**
   - BigQueryに保存されたトラフィックログの保持期間
   - デバイス側のローカルバッファ保持期間の推奨値

4. **複数IS-20Sの識別**
   - 同一施設に複数IS-20Sがある場合の識別方法
   - lacisIdで十分か？追加の識別子が必要か？

---

## 7. 次のステップ

1. **責務境界の確定**（AraneaDeviceGateチームとの協議）
2. **上記質問事項への回答**
3. **詳細API仕様書の作成**
4. **ペイロードスキーマの確定**

上記が明確になり次第、デバイス側の実装に着手可能です。

---

## 8. 参考: 現行IS-20S実装状況

| 機能 | 実装状況 | 備考 |
|------|----------|------|
| DNSトラフィック検知 | ○ 完了 | |
| HTTP/HTTPS/QUIC検知 | ○ 完了 | |
| domain_services辞書 | ○ 完了 | 445サービス登録済み |
| Unknown Domains検出 | ○ 完了 | FIFO 100件キャッシュ |
| イベント転送 | ○ 完了 | primary/secondary URL |
| バッチ送信 | △ 未実装 | 現行はイベント単位 |
| クラウド設定同期 | × 未実装 | 要仕様確定 |
| 辞書同期 | × 未実装 | 要仕様確定 |

---

**Aranea Device開発実装チーム**
