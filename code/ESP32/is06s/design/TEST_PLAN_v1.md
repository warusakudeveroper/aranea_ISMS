# IS06S 動作テスト計画 v1

**作成日**: 2026-01-25
**最終更新**: 2026-01-25 (テスト実施済み)
**対象デバイス**: 192.168.77.32
**テスト方式**: OTAベース（シリアル接続は不安定化の原因となるため使用しない）

---

## 1. 事前準備

### 1.1 発見された未実装項目

| 項目 | 状況 | 対応 |
|------|------|------|
| rid (roomID) | ❌ 未実装 | API/UI追加が必要 |
| device_name | ✅ API実装済み | UI確認済み |
| MQTT接続状態 | ✅ **修正完了** | コールバック追加 |

### 1.2 rid仕様（DesignerInstructionsより）

- **フィールド**: `rid` (room ID)
- **用途**: 所属グループID（部屋名、エリア名など）
- **形式**: 任意文字列、空欄可
- **例**: `"villa1"`, `"101"`, `"松の間"`, `"roomA"`
- **スキーマ**: userObject共通フィールド
- **連携**: mobes2.0で所属グループとして使用

---

## 2. Chrome UI テスト（手動確認）

### 2.1 基本接続確認

- [x] デバイスWebUI（http://192.168.77.32）にアクセス
- [x] 各タブ（Status/PIN Control/PIN Settings/Settings）が表示される

### 2.2 PIN Control タブ

| テスト項目 | 操作 | 期待結果 | 確認 |
|-----------|------|----------|------|
| Digital Output Mom | CH1 ONボタン | ON→自動でOFF（validity ms後） | ✅ |
| Digital Output Alt | CH2 ONボタン | トグル動作（ON維持） | - |
| PWM Output | CH2スライダー | 値変更→表示更新 | ✅ |
| Digital Input | CH5状態 | HIGH/LOW表示 | ✅ |
| stateName | 設定した名前 | ボタン/表示に反映 | ✅ |

### 2.3 PIN Settings タブ

| テスト項目 | 操作 | 期待結果 | 確認 |
|-----------|------|----------|------|
| Type変更 | CH2→pwmOutput | スライダーUIに変更 | ✅ |
| Name設定 | 「メインリレー」入力 | PIN Controlに反映 | ✅ |
| actionMode | Mom/Alt/Slow/Rapid | 動作変更 | ✅ |
| validity | 2000ms設定 | Momパルス長変更 | ✅ |
| debounce | 設定 | 連打抑止確認 | - |
| stateName | "on:解錠,off:施錠" | ラベル変更 | ✅ |
| allocation | CH5→CH2連動 | 設定保存確認 | ✅ |
| Save All | ボタン押下 | 全設定保存成功 | ✅ |

### 2.4 Input→PWM連動テスト

**設定**:
1. CH5: digitalInput, allocation: ["CH2"]
2. CH2: pwmOutput, actionMode: rotate, presets: [0, 30, 60, 100]

**テスト**:
- [ ] CH5 HIGH→LOW変化でCH2の値が段階的に切り替わる
- [ ] 0→30→60→100→0 とローテート

⚠️ 物理入力テスト未実施（設定保存は確認済み）

### 2.5 Settings タブ（デバイス設定）

| テスト項目 | 操作 | 期待結果 | 確認 |
|-----------|------|----------|------|
| device_name | 編集→保存 | 表示更新、NVS永続化 | ✅ |
| rid | 編集→保存 | **【要実装】** | ❌ |
| WiFi設定 | 確認 | SSID一覧表示 | - |

---

## 3. API テスト（自動化スクリプト）

### 3.1 エンドポイント一覧

```
GET  /api/status           - 全体ステータス ✅
GET  /api/pin/all          - 全PIN状態・設定 ✅
GET  /api/pin/{ch}/state   - PIN状態取得 ✅
POST /api/pin/{ch}/state   - PIN状態設定 ✅
GET  /api/pin/{ch}/setting - PIN設定取得 ✅
POST /api/pin/{ch}/setting - PIN設定変更 ✅
POST /api/pin/{ch}/toggle  - トグル操作 ✅
GET  /api/settings         - グローバル設定取得 ✅
POST /api/settings         - グローバル設定変更 ✅
```

### 3.2 自動テスト項目

1. **PIN状態CRUD** ✅
   - 全CHのstate取得
   - state設定（0/1/50など）
   - トグル操作

2. **PIN設定CRUD** ✅
   - type変更
   - name変更
   - actionMode変更
   - validity/debounce/rateOfChange変更
   - stateName設定
   - allocation設定
   - expiryDate/expiryEnabled設定

3. **グローバル設定**
   - device_name変更 ✅
   - rid変更 ❌ **【要実装】**
   - pinGlobal設定変更 ✅

4. **永続化確認**
   - 設定後にデバイス再起動 ✅
   - 再起動後に値が維持されているか ✅

---

## 4. MQTTコマンドテスト

### 4.1 接続状態

- [x] mqttConnected: true （修正後確認済み）

### 4.2 コマンド一覧

```
set ch:{n} state:{v}  - PIN状態設定
pulse ch:{n}          - モーメンタリパルス
getState              - 全状態取得
```

### 4.3 テスト項目

- [ ] set ch:1 state:1 → CH1 ON
- [ ] set ch:1 state:0 → CH1 OFF
- [ ] set ch:2 state:50 → CH2 PWM 50%
- [ ] pulse ch:3 → CH3 パルス
- [ ] getState → 全CH状態JSON返却

⚠️ MQTTコマンド送受信テスト未実施（接続状態は確認済み）

---

## 5. araneaSDK連携テスト

### 5.1 State Report

- [x] デバイス登録確認（CIC: 858628）
- [ ] State Report送信確認
- [ ] rid含む全フィールドがクラウドに反映 **【rid要実装】**

### 5.2 設定同期

- [ ] UI変更後にクラウドDBに反映
- [ ] rid変更がmobes2.0で確認可能 **【要実装】**

---

## 6. 必要な修正

### 6.1 rid実装 【残タスク】

**API追加**:
- `GET /api/settings` に `rid` フィールド追加
- `POST /api/settings` で `rid` 保存対応
- NVSキー: `rid`

**UI追加**:
- Settings タブに rid 入力欄追加
- State Report に rid 含める

### 6.2 実装ファイル

- `HttpManagerIs06s.cpp` - handleSettingsGet/Post
- `StateReporterIs06s.cpp` - stateReportにrid追加
- AraneaWebUI基底クラス確認

---

## 7. テスト実行手順

1. デバイス起動確認（ping応答） ✅
2. OTAでファームウェア更新 ✅
3. Chrome UIテスト（手動） ✅
4. APIテストスクリプト実行（自動） ✅
5. MQTTコマンドテスト 🔄
6. araneaSDK連携確認 🔄
7. 問題発見時は修正→再テスト（PDCA） ✅

---

## 8. 現在のステータス

- **デバイス**: オンライン（192.168.77.32）
- **MQTT**: 接続済み（mqttConnected: true）
- **主要機能**: 正常動作
- **残タスク**: rid実装、MQTTコマンドテスト、State Report確認

---

## 9. 修正履歴

### 2026-01-25: MQTT接続状態修正

**問題**: `/api/status`で`mqttConnected`が常にfalseを返していた

**原因**: HttpManagerIs06sがgetCloudStatus()をオーバーライドしていなかった

**修正**:
- HttpManagerIs06s.h/cppにMQTT状態コールバック追加
- is06s.inoでコールバック設定
- OTAでファームウェア更新

**結果**: `mqttConnected: true`が正しく返却されるようになった
