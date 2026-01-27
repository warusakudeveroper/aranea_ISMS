# IS06S 設計ドキュメントインデックス

**製品名**: Aranea Relay & Switch Controller
**製品コード**: AR-IS06S
**作成日**: 2025/01/23
**最終更新**: 2026/01/25
**ステータス**: **実装完了・本番運用可能**

---

## 1. ドキュメント一覧

| No | ドキュメント名 | ファイル | 目的 | ステータス |
|----|---------------|----------|------|----------|
| 1 | 設計インデックス | Design_Index.md | 全体ナビゲーション | ✅ 完了 |
| 2 | 基本仕様書 | IS06S_BasicSpecification.md | 製品概要・PIN仕様・データ構造 | ✅ 完了 |
| 3 | 詳細設計書 | IS06S_DetailedDesign.md | MECE・アンアンビギュアスな実装設計 | ✅ 完了 |
| 4 | モジュール適応計画 | IS06S_ModuleAdaptationPlan.md | 共通モジュール使用・固有モジュール設計 | ✅ 完了 |
| 5 | タスクリスト | IS06S_TaskList.md | 実装タスク・依存関係・進捗管理 | ✅ 完了 |
| 6 | テスト計画 | IS06S_TestPlan.md | フロントエンド・バックエンド・UIテスト | ✅ 完了 |
| 7 | API仕様書 | API_GUIDE.md | HTTP APIエンドポイント仕様 | ✅ 完了 |
| 8 | 実装状況報告書 | IMPLEMENTATION_REPORT.md | 実装進捗・レビュー対応 | ✅ 完了 |
| 9 | テスト結果報告書 | TEST_RESULTS_v1.md | 総合テスト結果 | ✅ 完了 |
| 10 | **実装完了報告書** | IS06S_COMPLETION_REPORT.md | 最終実装完了報告・運用開始判定 | ✅ **完了** |

### レビュードキュメント

| No | ドキュメント名 | ファイル | 目的 | ステータス |
|----|---------------|----------|------|----------|
| R1 | Designer Review #1 | review/designersreview1.md | デザイナーレビュー指摘事項 | ✅ 対応済み |
| R2 | DR1対応報告 | review/designersreview1_response.md | レビュー対応詳細 | ✅ 完了 |
| R3 | DR1テスト結果 | review/DR1_TEST_RESULTS.md | Chrome実機照合結果 | ✅ 14/14 PASS |

---

## 2. 製品概要

### 2.1 IS06Sとは

IS06SはaraneaDeviceシリーズのリレー・スイッチコントローラです。

**主要機能**:
- 4ch Digital/PWM出力（CH1〜CH4）
- 2ch Digital入出力（CH5〜CH6）
- WiFi接続（最大6 SSID対応）
- APモード対応
- MQTT/HTTP制御
- I2C OLED表示
- HTTP OTA更新

**主用途**:
- 自動販売機制御
- 電磁錠（施錠/解錠）
- 照明ON/OFF
- 調光照明（PWM）
- 汎用リレー制御

### 2.2 ハードウェア基盤

```json
{
  "hardware components": {
    "SBC": "ESP32Wroom32(ESP32E,DevkitC)",
    "I2COLED": "0.96inch_128*64_3.3V",
    "baseboard": "aranea_04"
  }
}
```

### 2.3 関連デバイス

| デバイス | 概要 | IS06Sとの関係 |
|---------|------|--------------|
| is04a | 2ch入出力コントローラ | IS06Sの2ch限定版 |
| is05a | 8chリード入力 | 入力専用、IS06SはI/O兼用 |
| is06a | 6ch出力（PWM対応） | IS06Sの前身設計 |

---

## 3. アーキテクチャ概要

### 3.1 ディレクトリ構成

```
code/ESP32/
├── global/src/              ← 共通モジュール（変更なし）
│   ├── WiFiManager.h/cpp
│   ├── SettingManager.h/cpp
│   ├── DisplayManager.h/cpp
│   ├── NtpManager.h/cpp
│   ├── MqttManager.h/cpp
│   ├── AraneaWebUI.h/cpp
│   ├── HttpOtaManager.h/cpp
│   ├── Operator.h/cpp
│   ├── LacisIDGenerator.h/cpp  ← 必須
│   └── AraneaRegister.h/cpp    ← 必須
│
└── is06s/                   ← IS06S固有
    ├── is06s.ino            ← メインスケッチ
    ├── AraneaGlobalImporter.h ← 共通モジュールインポート
    ├── Is06PinManager.h/cpp ← PIN管理（本製品のコア）
    ├── HttpManagerIs06s.h/cpp ← Web UI
    ├── StateReporterIs06s.h/cpp ← 状態送信
    └── design/              ← 設計ドキュメント（本ディレクトリ）
```

### 3.2 PIN構成概要

| PIN | 機能 | Type |
|-----|------|------|
| GPIO18 | CH1 | D/P (Digital/PWM Output) |
| GPIO05 | CH2 | D/P (Digital/PWM Output) |
| GPIO15 | CH3 | D/P (Digital/PWM Output) |
| GPIO27 | CH4 | D/P (Digital/PWM Output) |
| GPIO14 | CH5 | I/O (Digital Input/Output) |
| GPIO12 | CH6 | I/O (Digital Input/Output) |
| GPIO25 | Reconnect | System (WiFi再接続) |
| GPIO26 | Reset | System (ファクトリーリセット) |
| GPIO21 | I2C SDA | I2C |
| GPIO22 | I2C SCL | I2C |

---

## 4. The_golden_rules.md遵守事項

### 4.1 大原則との対応

| 原則 | IS06S設計での対応 |
|------|------------------|
| 1. SSoTの大原則 | userObjectDetailをSSoTとしMQTT/NVS同期 |
| 2. SOLID原則 | Is06PinManagerで単一責任、依存逆転を遵守 |
| 3. MECE | PIN機能をD/P、I/O、Systemで漏れなく分類 |
| 4. アンアンビギュアス | 全設定項目の型・デフォルト値・範囲を明示 |
| 5. 情報等価 | 全PIN状態をWebUI/MQTT両方から参照可能 |
| 6. 現場猫禁止 | ハードコード禁止、設定はNVS/userObjectDetail |
| 12. 車輪の再発明禁止 | global共通モジュールを最大限活用 |
| 15. 設計ドキュメント必須 | 本インデックス含む6ドキュメント整備 |
| 19. araneaSDK使用 | AraneaDeviceGate登録・認証にSDK使用 |

### 4.2 MECE確認

本設計ドキュメント群は以下の観点でMECE（漏れなくダブりなく）です：

1. **機能分類**: D/P（出力）、I/O（入出力）、System（システム）で全PINをカバー
2. **ドキュメント分類**: 概要→詳細→モジュール→タスク→テストで設計〜実装〜検証をカバー
3. **データ構造**: PINSettings（設定）、PINState（状態）、GlobalSettings（全体設定）で全設定をカバー

---

## 5. 依存関係

### 5.1 外部依存

| 依存先 | 用途 |
|--------|------|
| AraneaDeviceGate | デバイス登録・CIC取得 |
| araneaSDK | スキーマ検証・認証 |
| mobes2.0 | userObjectDetail同期 |

### 5.2 内部依存（共通モジュール）

| モジュール | 必須 | 用途 |
|-----------|------|------|
| LacisIDGenerator | **必須** | lacisID生成 |
| AraneaRegister | **必須** | CIC取得 |
| WiFiManager | ○ | WiFi接続管理 |
| SettingManager | ○ | NVS永続化 |
| MqttManager | ○ | MQTT接続 |
| DisplayManager | ○ | OLED表示 |
| NtpManager | ○ | NTP同期 |
| AraneaWebUI | ○ | Web UI基底 |
| HttpOtaManager | ○ | OTA更新 |

---

## 6. 参照

- **is04a設計**: `code/ESP32/is04a/design/`
- **is05a設計**: `code/ESP32/is05a/design/`
- **共通モジュール**: `code/ESP32/global/src/`
- **The_golden_rules**: `/The_golden_rules.md`
- **araneaSDK**: MCP `aranea_*` ツール群

---

## 7. 変更履歴

| 日付 | 変更内容 | 担当 |
|------|----------|------|
| 2025/01/23 | 初版作成（Design_Index.md） | Claude |
| 2026/01/25 | 実装完了報告書追加、ステータス更新 | Claude Code |
