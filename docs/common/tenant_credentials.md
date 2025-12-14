# テナント認証情報（ISMS プロジェクト共通）

**注意**: このファイルには認証情報が含まれています。本番環境では適切に管理してください。

---

## テナント情報

| 項目 | 値 |
|------|-----|
| テナント名 | 市山水産株式会社 |
| TID | `T2025120608261484221` |
| FID | `9000` |

---

## テナントプライマリ（permission 61）

| 項目 | 値 |
|------|-----|
| lacisID | `12767487939173857894` |
| Email (userId) | `info+ichiyama@neki.tech` |
| CIC | `263238` |
| Password | `dJBU^TpG%j$5` |

---

## 用途

### araneaDeviceGate 登録時
```json
{
  "lacisOath": {
    "lacisId": "12767487939173857894",
    "userId": "info+ichiyama@neki.tech",
    "pass": "dJBU^TpG%j$5",
    "cic": "263238",
    "method": "register"
  },
  "userObject": {
    "tid": "T2025120608261484221",
    "typeDomain": "araneaDevice",
    "type": "ISMS_ar-is02"
  },
  "deviceMeta": {
    "macAddress": "<12桁MAC>",
    "productType": "002",
    "productCode": "0096"
  }
}
```

### deviceStateReport 認証時
```json
{
  "auth": {
    "tid": "T2025120608261484221",
    "lacisId": "<デバイスのlacisId>",
    "cic": "<登録時に発行されたCIC>"
  }
}
```

---

## 関連ドキュメント

- [external_systems.md](./external_systems.md) - mobes2.0 API仕様
- [AraneaRegister モジュール](../../code/ESP32/global/src/AraneaRegister.h)
