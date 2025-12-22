# is04a テスト待ち事項

**作成日**: 2024-12-22
**ステータス**: araneaDeviceGate準備待ち

---

## 実装完了項目

### 新規実装（2024-12-22 完了）

| ファイル | 説明 |
|----------|------|
| Is04aKeys.h | NVSキー定数（15文字制限対応） |
| AraneaSettings.h/cpp | 施設展開用デフォルト設定 |
| TriggerManager.h/cpp | 2ch入力 + 2ch出力連動管理 |
| StateReporterIs04a.h/cpp | 状態レポート構築・送信 |
| HttpManagerIs04a.h/cpp | Web UI実装 |
| AraneaGlobalImporter.h | 共通モジュールインポート |
| is04a.ino | メインスケッチ |

### ビルド結果

- Flash: 65% (1,285,113 bytes)
- RAM: 16% (53,584 bytes)
- パーティション: min_spiffs

---

## テスト実施予定

araneaDeviceGateが準備でき次第、以下のテストを実施する。

### 1. 基本動作テスト

- [ ] 物理入力検知（GPIO5, GPIO18 デバウンス付き）
- [ ] パルス出力（GPIO12, GPIO14）
- [ ] トリガーアサイン動作（入力1→出力1, 入力2→出力2）
- [ ] インターロック動作（連続パルス防止）
- [ ] Web UI表示・操作
- [ ] Basic認証動作

### 2. API動作テスト

- [ ] `GET /api/status` - 入出力状態取得
- [ ] `POST /api/pulse` - パルス実行（output, duration指定）
- [ ] `GET /api/config` - 設定取得
- [ ] `POST /api/config` - 設定変更
- [ ] `GET /api/trigger` - トリガーアサイン取得
- [ ] `POST /api/trigger` - トリガーアサイン変更

### 3. StateReporter動作テスト

- [ ] ローカル送信（192.168.96.201, 192.168.96.202）
- [ ] クラウド送信（tid/cic認証付き）
- [ ] パルス終了時の状態送信
- [ ] 入力変化時の状態送信

### 4. 表示・UX テスト

- [ ] OLED表示（IP, CIC, 電波強度）
- [ ] パルス実行中の表示（CLOUD/MANUAL/HTTP + 出力名）
- [ ] WiFi再接続ボタン（3秒長押し）
- [ ] ファクトリーリセットボタン（3秒長押し）
- [ ] APモード動作（WiFi接続失敗時）

---

## GPIO割り当て

| GPIO | 機能 | 説明 |
|------|------|------|
| 5 | PHYS_IN1 | 物理入力1（INPUT_PULLDOWN） |
| 18 | PHYS_IN2 | 物理入力2（INPUT_PULLDOWN） |
| 12 | TRG_OUT1 | トリガー出力1（OPEN） |
| 14 | TRG_OUT2 | トリガー出力2（CLOSE） |
| 21 | I2C_SDA | OLED SDA |
| 22 | I2C_SCL | OLED SCL |
| 25 | BTN_WIFI | WiFi再接続（3秒長押し） |
| 26 | BTN_RESET | ファクトリーリセット（3秒長押し） |

---

## デフォルト設定

| 項目 | 値 |
|------|-----|
| パルス幅 | 3000ms |
| インターロック | 200ms |
| デバウンス | 50ms |
| 入力1→出力 | OUT1 |
| 入力2→出力 | OUT2 |
| 出力1名 | OPEN |
| 出力2名 | CLOSE |

---

## 将来対応予定

- [ ] MQTT受信/制御（クラウドからのコマンド）
- [ ] OTAリモート更新

---

## araneaDeviceGate準備後の手順

1. is04aデバイスをネットワークに接続
2. シリアルモニタで起動確認
3. Web UI（http://<device-ip>/）にアクセス
4. 上記テスト項目を順次実施
5. 問題があれば修正、再コンパイル・アップロード

---

## 関連ドキュメント

- [design/DESIGN.md](../../code/ESP32/is04a/design/DESIGN.md) - 設計書
- [design/MODULE_ADAPTATION_PLAN.md](../../code/ESP32/is04a/design/MODULE_ADAPTATION_PLAN.md) - モジュール適応計画
- [../common/tenant.md](../common/tenant.md) - テナント情報
