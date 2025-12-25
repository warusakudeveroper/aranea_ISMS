# AraneaSDK Design Documents

## Overview

AraneaSDKは、mobes2.0とaraneaDeviceの連携を確実に行うための開発ツールキットです。
両プロジェクト間でデータ形式、認証仕様、スキーマの正本を共有し、実装の整合性を担保します。

**重要**: このドキュメントは mobes2.0 側と aranea_ISMS 側で同期され、両チームが同一のルールブックを参照します。

---

## Document Index

| ドキュメント | 内容 | 対象者 |
|------------|------|--------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | システム全体アーキテクチャ | 全員 |
| [AUTH_SPEC.md](./AUTH_SPEC.md) | 認証・認可仕様（lacisOath） | 全員 |
| [SCHEMA_SPEC.md](./SCHEMA_SPEC.md) | スキーマ定義・型システム | 全員 |
| [API_REFERENCE.md](./API_REFERENCE.md) | API エンドポイント仕様 | ファーム/バックエンド |
| [DEVICE_IMPLEMENTATION.md](./DEVICE_IMPLEMENTATION.md) | デバイスファームウェア実装ガイド | ファームウェア開発者 |
| [VALIDATION_TOOLS.md](./VALIDATION_TOOLS.md) | 検証ツール仕様 | 全員 |
| [DEVELOPMENT_WORKFLOW.md](./DEVELOPMENT_WORKFLOW.md) | 開発ワークフロー | 全員 |
| [TYPE_REGISTRY.md](./TYPE_REGISTRY.md) | ProductType/Type登録仕様 | 全員 |

---

## Version History

| バージョン | 日付 | 変更内容 |
|-----------|------|---------|
| 0.1.0 | 2025-12-25 | 初版作成 |

---

## Repository Links

- **mobes2.0**: https://github.com/warusakudeveroper/mobes2.0
- **aranea_ISMS**: https://github.com/warusakudeveroper/aranea_ISMS

---

## Quick Reference

### LacisID Format (20文字固定)
```
[Prefix=3][ProductType=3桁][MAC=12HEX][ProductCode=4桁]
例: 3 004 AABBCCDDEEFF 0001
```

### Current ProductTypes
| ProductType | Device | Description |
|-------------|--------|-------------|
| 001 | is01 | 電池式温湿度センサー |
| 002 | is02 | WiFiゲートウェイ |
| 003 | is03 | Orange Pi Zero3 |
| 004 | is04 | 2ch入出力コントローラー |
| 005 | is05 | 8ch検出器 |
| 006 | is06 | 6ch出力（PWM対応） |
| 010 | is10 | ルーター検査デバイス |
| 020 | is20 | ネットワーク監視 |

### Default Tenant (市山水産株式会社)
```
TID: T2025120608261484221
FID: 9000
Tenant Primary LacisID: 12767487939173857894
Email: info+ichiyama@neki.tech
CIC: 263238
```

### Endpoints
| Purpose | URL |
|---------|-----|
| DeviceGate | https://us-central1-mobesorder.cloudfunctions.net/araneaDeviceGate |
| StateReport | https://us-central1-mobesorder.cloudfunctions.net/deviceStateReport |
| MQTT Bridge | wss://aranea-mqtt-bridge-*.run.app |

---

## Contact

- **mobes2.0 Team**: mobes2.0リポジトリのIssues
- **araneaDevice Team**: aranea_ISMSリポジトリのIssues
