# is05a テスト待ち事項

**作成日**: 2024-12-22
**ステータス**: araneaDeviceGate準備待ち

---

## 実装完了項目

### コードレビュー修正（2024-12-22 完了）

| # | 修正内容 | コミット |
|---|----------|----------|
| 1 | `<script>`タグ追加（generateTypeSpecificJS） | 88236b2 |
| 2 | APIパス統一（/api/channel + body内ch） | 88236b2 |
| 3 | is05a固有エンドポイントにcheckAuth()追加 | 88236b2 |
| 4 | JSON生成をArduinoJson使用に変更 | 88236b2 |
| 5 | デバウンスを経過時間ベースに修正 | 88236b2 |

---

## テスト実施予定

araneaDeviceGateが準備でき次第、以下のテストを実施する。

### 1. 基本動作テスト

- [ ] 8ch入力検知（リードスイッチ開閉）
- [ ] デバウンス動作確認（100ms設定で実際に100ms待機するか）
- [ ] Web UI表示・操作
- [ ] Basic認証動作

### 2. API動作テスト

- [ ] `GET /api/channels` - 全チャンネル状態取得
- [ ] `GET /api/channel?ch=1` - 個別チャンネル状態取得
- [ ] `POST /api/channel` - ch7/ch8モード切替
- [ ] `POST /api/channel/config` - チャンネル設定変更
- [ ] `POST /api/pulse` - パルス出力（ch7/ch8出力モード時）

### 3. StateReporter動作テスト

- [ ] ローカル送信（192.168.96.201, 192.168.96.202）
- [ ] クラウド送信（tid/cic認証付き）
- [ ] 状態変化時の即座送信

### 4. Webhook動作テスト

- [ ] Discord Webhook送信
- [ ] Slack Webhook送信
- [ ] Generic Webhook送信
- [ ] `/api/webhook/test` テスト送信

---

## 未対応のOpen Points

テスト後、必要に応じて対応を検討する。

### OP-1: /api/webhook/test のplatformパラメータ

**現状**: platformパラメータを無視し、設定された全URLにテスト送信

**推奨対応**: platform別テスト実装
- Discord/Slack/Genericそれぞれのテストボタンで個別URLにのみ送信
- 設定ミスの切り分けが容易になる

**実装案**:
```cpp
void WebhookManager::sendTest(Platform platform) {
    switch (platform) {
        case Platform::DISCORD:
            if (discordUrl_.length() > 0) {
                sendToUrl(discordUrl_, buildDiscordPayload(1, true, timestamp));
            }
            break;
        case Platform::SLACK:
            // ...
    }
}
```

### OP-2: ハートビート送信間隔

**現状**: `sendHeartbeat()`が`sendStateReport()`と同じ1秒間隔制限を共有

**推奨対応**: 現状維持（省略可）
- 状態変化レポート送信＝通信できている証拠
- 別カウンタは複雑化を招く
- ハートビート間隔（5分等）は呼び出し側で管理

---

## araneaDeviceGate準備後の手順

1. is05aデバイスをネットワークに接続
2. シリアルモニタで起動確認
3. Web UI（http://<device-ip>/）にアクセス
4. 上記テスト項目を順次実施
5. 問題があれば修正、再コンパイル・アップロード
6. Open Pointsの対応要否を判断

---

## 関連ドキュメント

- [REVIEW_REQUEST.md](./REVIEW_REQUEST.md) - 実装レビュー依頼書
- [../common/tenant.md](../common/tenant.md) - テナント情報

---

## 連絡先

テスト準備が整い次第、本ドキュメントを更新してテスト結果を記録すること。
