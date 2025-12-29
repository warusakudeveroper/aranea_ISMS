# 3者統合チェックリスト回答書

**From**: Aranea Device開発実装チーム
**Date**: 2025-12-29
**Subject**: IS-10 / IS-10m チェックリスト確認結果と修正計画

---

## 1. IS-10 (Router Inspector) 確認結果

### Aranea Device側チェック項目

| # | 項目 | 現状 | 対応 |
|---|-----|:----:|------|
| 1 | LacisID形式: `4010{MAC12}{ProductCode4}` | ⚠️ | 現状 `3010{MAC12}{0001}` → **要修正** |
| 2 | sourceType: `ar-is10` | ☑ | `aranea_ar-is10` で送信中 |
| 3 | auth.lacisId, auth.cic 送信 | ☑ | StateReporterIs10.cpp:143-145 |
| 4 | report.router.mac (ルーターMAC) | ❌ | **未実装**: 現在 `reportedConfig.is10.routers[]` に設定情報のみ |
| 5 | report.router.wanIp, lanIp | ❌ | **未実装**: SshPollerで取得済みだがレポートに含まれていない |
| 6 | report.clients[] (接続クライアントリスト) | ❌ | **未実装**: clientCount（数）のみ、リストは取得していない |
| 7 | connected: true のクライアントのみ送信 | ❌ | クライアントリスト自体が未実装 |
| 8 | 5分間隔 + イベント駆動送信 | ⚠️ | 間隔は設定可能（デフォルト60秒）、イベント駆動未実装 |

### 現在のIS-10送信ペイロード構造

```json
{
  "auth": {
    "tid": "T2025...",
    "lacisId": "3010F8B3B7...",
    "cic": "560848"
  },
  "report": {
    "lacisId": "3010F8B3B7...",
    "type": "aranea_ar-is10",
    "observedAt": "2025-12-29T...",
    "state": {
      "deviceName": "is10-xxx",
      "IPAddress": "192.168.x.x",
      "routerCount": 4,
      "reportedConfig": {
        "is10": {
          "scanInterval": 60,
          "routers": [{ "rid": "R101", "ipAddr": "...", ... }]
        }
      }
    }
  }
}
```

### チェックリスト要求ペイロード構造

```json
{
  "auth": { "tid", "lacisId", "cic" },
  "report": {
    "lacisId": "4010{MAC12}{ProductCode4}",
    "type": "ar-is10",
    "router": {
      "mac": "AABBCCDDEEFF",
      "wanIp": "xxx.xxx.xxx.xxx",
      "lanIp": "192.168.x.1"
    },
    "clients": [
      { "mac": "...", "hostname": "...", "ip": "...", "connected": true }
    ]
  }
}
```

### 修正計画

1. **LacisID形式変更**: `3010` → `4010`
2. **sourceType簡略化**: `aranea_ar-is10` → `ar-is10`
3. **report.router追加**: SshPollerで取得済みのRouterInfo構造体からマッピング
4. **report.clients[]追加**: DHCPリースからクライアントリスト取得（新規実装必要）
5. **イベント駆動送信**: ルーター状態変化時のトリガー追加

---

## 2. IS-10m (Mercury AC Inspector) 確認結果

### ステータス: **未実装**

`code/ESP32/is10m/` ディレクトリは存在しません。

### 参照可能な設計資料

- `docs/is10m/API_GUIDE.md` - Mercury AC API仕様
- `docs/is10m/DESIGN.md` - is10m設計書
- `docs/is10m/MODULE_ADAPTATION_PLAN.md` - 既存モジュール適応計画

### 実装計画

IS-10の実装をベースに、以下を変更：

| 項目 | IS-10 | IS-10m |
|-----|-------|--------|
| データ取得先 | SSH (OpenWrt/AsusWRT) | HTTP API (Mercury AC) |
| 認証方式 | SSH Password/Key | XORエンコードパスワード + セッショントークン |
| AP/クライアント取得 | DHCPリース解析 | `/stm/client/list`, `/stm/ap/list` |
| sourceType | ar-is10 | ar-is10m |

---

## 3. CIC認証仕様の確認

### チェックリスト記載

```
CIC = HMAC-SHA256(secret, lacisId + mac + timestamp + nonce)
```

### 現行実装

```cpp
// AraneaRegister.cpp
result.cic_code = resDoc["userObject"]["cic_code"].as<String>();
// CICはaraneaDeviceGateから取得した6桁コード
```

### 確認事項

チェックリストには「CIC生成ルール」としてHMAC-SHA256が記載されていますが、現行実装ではCICは**araneaDeviceGateからの登録レスポンスで取得する6桁コード**です。

**質問**: CIC認証は以下のどちらですか？

1. **登録時取得**: araneaDeviceGateから取得した6桁CICをそのまま送信（現行実装）
2. **リクエスト時生成**: 各リクエストでHMAC-SHA256を動的生成

現行実装（選択肢1）で問題ないか確認をお願いします。

---

## 4. 署名

| チーム | 担当者 | 日付 | 署名 |
|-------|-------|------|:----:|
| CelestialGlobe実装班 | - | 2025-12-29 | ☑ |
| AraneaDeviceGate/AraneaSDK班 | - | 2025-12-29 | ☑ |
| **Aranea Device開発実装チーム** | - | 2025-12-29 | **☑ 確認完了** |

---

## 5. 次のアクション

### 優先度: 高

1. [ ] IS-10 クライアントリスト取得機能の追加
2. [ ] IS-10 report構造の仕様書準拠への変更
3. [ ] IS-10 LacisID形式の変更 (`3010` → `4010`)

### 優先度: 中

4. [ ] IS-10m 基本実装開始
5. [ ] イベント駆動送信の実装

### 確認待ち

6. [ ] CIC認証方式の最終確認

---

**Aranea Device開発実装チーム**
