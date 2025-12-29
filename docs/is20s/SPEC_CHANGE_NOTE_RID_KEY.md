# IS-20S統合仕様書 変更メモ

**Date**: 2025-12-29
**Subject**: 部屋識別キーの変更（room_no → rid）

---

## 変更内容

### 変更前（最終仕様書）

```typescript
interface Is20sTrafficPacket {
  room_no: string;  // 部屋番号
}
```

### 変更後（実装）

```typescript
interface Is20sTrafficPacket {
  rid: string;  // 部屋ID（Room ID）
}
```

---

## 変更理由

**lacisOathのuserObjectスキーマ共通仕様に準拠**

- lacisOath系（既存のAraneaDevice認証・登録システム）では `rid` キーで部屋IDを管理
- 既存システムとの整合性を維持するため `room_no` ではなく `rid` を使用
- `rid` は自由書式の文字列（数字、文字列、いずれも許容）

---

## 影響範囲

### デバイス側（IS-20S）

- `ingest_client.py`: `Is20sTrafficPacket` クラスで `rid` を使用
- `api.py`: `/api/ingest/room-mapping` エンドポイントで `rid` を返却

### クラウド側（CelestialGlobe）

- `is20sHandler.ts`: パケット解析時に `rid` フィールドを読み取る必要あり
- BigQueryスキーマ: `room_no` → `rid` に変更が必要

---

## 仕様書更新依頼

以下のドキュメントの更新が必要です：

| ドキュメント | 変更箇所 |
|-------------|---------|
| IS20S_FINAL_INTEGRATION_SPEC.md | §3.1 Is20sTrafficPacketスキーマ |

### 更新内容

```diff
interface Is20sTrafficPacket {
  ts: string;
  src_ip: string;
  dst_ip: string;
  port: number;
  protocol: 'TCP' | 'UDP';
- room_no: string;         // 部屋番号（必須）
+ rid: string;             // 部屋ID（lacisOath userObject準拠）
```

### 追加コメント

```
※ ridキーはlacisOathのuserObjectスキーマ共通仕様に準拠
```

---

## 対応状況

- [x] デバイス側実装完了（ingest_client.py）
- [ ] 仕様書更新（CelestialGlobe実装班に依頼）
- [ ] クラウド側対応（Phase A実装時）

---

**Aranea Device開発実装チーム**
