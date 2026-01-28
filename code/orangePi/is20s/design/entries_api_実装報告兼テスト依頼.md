# IS20S `GET /api/capture/entries` 実装完了報告 兼 テスト依頼書

**日付**: 2026-01-29
**対象Issue**: https://github.com/warusakudeveroper/aranea_ISMS/issues/124
**設計書**: `docs/design/IS20S_TRAFFIC_ENTRY_API.md`
**デプロイ先**: soropia IS20S `192.168.77.90:8080`

---

## 1. 実装概要

Issue #124 で設計した `GET /api/capture/entries` エンドポイントをIS20S側に実装・デプロイ完了。
NDJSONログファイルから個別トラフィックエントリを読み出し、OSノイズフィルタ適用・カーソルベースページネーション対応のAPIを提供する。

## 2. 修正ファイル

| ファイル | 変更内容 |
|----------|----------|
| `code/orangePi/is20s/app/capture.py` | `is_os_noise()`, `map_event_to_entry()`, `CaptureFileLogger.read_entries()` 追加 |
| `code/orangePi/is20s/app/api.py` | `GET /api/capture/entries` ルート追加 |

既存APIへの変更: **なし**（`/api/capture/statistics`, `/api/capture/events` は完全に独立）

## 3. APIエンドポイント仕様

### `GET /api/capture/entries`

| パラメータ | 型 | デフォルト | 説明 |
|------------|------|-----------|------|
| `since` | string | null | カーソル（エントリID or ISO8601タイムスタンプ） |
| `limit` | int | 1000 | 取得件数上限（1-5000） |
| `primary_only` | bool | false | auxiliary通信を除外 |

### レスポンス

```json
{
  "ok": true,
  "entries": [
    {
      "id": "villa220260127125841-6890",
      "timestamp": "2026-01-27T12:46:20.446652+09:00",
      "room": "villa2",
      "category": "Messenger",
      "service": "Discord",
      "count": 1,
      "src_ip": "192.168.20.4",
      "dst_ip": "162.159.129.233",
      "dst_domain": "cdn.discordapp.com",
      "protocol": "tcp",
      "dst_port": 443,
      "bytes": null
    }
  ],
  "has_more": true,
  "next_cursor": "villa220260127125841-9537",
  "total_returned": 5,
  "filter_applied": {
    "primary_only": false,
    "os_noise_excluded": true
  }
}
```

### OSノイズフィルタ（常時適用、11項目）

| # | フィルタ | 対象 |
|---|---------|------|
| F1-F7,F11 | ポートベース | 123(NTP), 67/68(DHCP), 5353(mDNS), 5355(LLMNR), 1900(SSDP), 137-139(NetBIOS) |
| F3,F10 | プロトコルベース | ARP, IGMP |
| F8 | OS自動更新 | windowsupdate.com, update.microsoft.com, swscan.apple.com 等 |
| F9 | OCSP/CRL | ocsp.*, crl.* |

## 4. デプロイ済み動作確認結果

soropia IS20S (192.168.77.90) でスモークテスト実施。

| # | テスト項目 | 結果 | 備考 |
|---|-----------|------|------|
| T1 | 基本取得 `?limit=5` | OK | 5件取得、has_more=true |
| T2 | ページネーション `?since={cursor}&limit=3` | OK | page2で3件取得、next_cursor発行 |
| T3 | primary_onlyフィルタ `?primary_only=true` | OK | 100件取得、カテゴリ: AI/Auth/Cloud/IoT/Messenger/Tracker |
| T4 | OSノイズ除外確認 | OK | 500件中ノイズポート(123,67,68等)含有: 0件 |
| T5 | 既存API `/api/capture/statistics` | OK | total_events=23174、変更なし |

## 5. sorapiapps側テスト依頼

### 5.1 接続先

```
エンドポイント: http://192.168.77.90:8080/api/capture/entries
```

### 5.2 テスト項目

sorapiapps側での確認をお願いします。

| # | テスト項目 | テスト手順 | 期待結果 |
|---|-----------|-----------|---------|
| S1 | 基本取得 | `GET /api/capture/entries?limit=10` | `ok: true`, entries配列に10件、has_more=true |
| S2 | カーソルページネーション | S1の`next_cursor`を`since`に指定して再リクエスト | 前回と重複しない次のエントリが返る |
| S3 | 全件走査 | `since`を繰り返し指定して`has_more=false`まで取得 | 全エントリを漏れなく取得できる |
| S4 | primary_onlyフィルタ | `?primary_only=true` | auxiliary通信（CDN、infrastructure系）が除外される |
| S5 | OSノイズ除外 | レスポンス内のdst_portを検査 | 123,67,68,5353,5355,1900,137-139が含まれない |
| S6 | traffic-poller.js統合 | pollerからentriesを定期取得しDBに保存 | カーソル管理が正常に動作しデータが蓄積される |
| S7 | 既存API非影響 | `/api/capture/statistics`を呼び出し | 従来通りの集計レスポンスが返る |
| S8 | エントリフィールド検証 | 各エントリのフィールドを検査 | 全12フィールド(id,timestamp,room,category,service,count,src_ip,dst_ip,dst_domain,protocol,dst_port,bytes)が存在 |

### 5.3 traffic-poller.js側の統合イメージ

```javascript
// カーソル管理: 前回のnext_cursorを保存し、次回sinceに指定
const cursor = await db.get("SELECT cursor FROM traffic_cursors WHERE device_ip = ?", [deviceIp]);
const since = cursor ? `&since=${cursor.cursor}` : "";
const res = await fetch(`http://${deviceIp}:8080/api/capture/entries?limit=1000${since}`);
const data = await res.json();

if (data.ok && data.entries.length > 0) {
  // エントリをDB保存
  for (const entry of data.entries) {
    await db.run("INSERT INTO traffic_entries ...", [entry.id, entry.timestamp, ...]);
  }
  // カーソル更新
  if (data.next_cursor) {
    await db.run("UPDATE traffic_cursors SET cursor = ? WHERE device_ip = ?", [data.next_cursor, deviceIp]);
  }
}
```

### 5.4 注意事項

- `bytes`フィールドは常に`null`（tsharkのメタデータキャプチャではバイト数取得不可）
- `count`フィールドは常に`1`（個別エントリのため）
- `next_cursor`は`has_more=true`の場合のみ値が入る
- `since`にISO8601タイムスタンプを指定した場合はepoch比較で走査開始（エントリIDより低精度）
- 最大走査量: 10ファイル/10MB（Orange Pi上で100ms以下の応答速度）

## 6. 大原則履行報告

- **SSoT遵守**: NDJSONファイルログが唯一のデータソース。display_bufferからは読まない
- **SOLID**: `is_os_noise()`、`map_event_to_entry()`は単一責任関数。`read_entries()`は既存`aggregate_statistics()`と並列
- **MECE**: OSノイズフィルタ11項目（ポート9種+プロトコル2種+ドメイン2カテゴリ）が重複なく網羅
- **アンアンビギュアス**: API仕様のパラメータ・レスポンス・フィルタ条件が全て明確定義
- **差別コード禁止**: 全トラフィック情報は等価に記録（NDJSONログ）。APIレスポンスのみOSノイズ除外（明確な技術基準）
- **車輪の再発明禁止**: 既存の`CaptureFileLogger`クラスにメソッド追加。`_get_log_files()`等を再利用
- **現場猫案件禁止**: 実機デプロイ+5項目スモークテスト実施済み
