# 統合テスト計画: ffmpegリーク修正

**作成日**: 2026-01-05 12:45 JST
**親ドキュメント**: [FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md](./FIX_DESIGN_ffmpeg_leak_and_frontend_boundary.md)

---

# 絶対ルール（大原則の宣言）

**以下を必ず復唱してからテストを開始すること**

1. SSoTの大原則を遵守します
2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意します
3. 必ずMECEを意識し、このコードはMECEであるか、この報告はMECEであるかを報告の中に加えます。
4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行います。
5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報の公平性原則
6. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻ります
7. チェック、テストのない完了報告はただのやった振りのためこれは行いません
8. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行いません
9. 作業のフェーズ開始前とフェーズ終了時に必ずこの実施手順と大原則を全文省略なく復唱します
10. 必須参照の改変はゴールを動かすことにつながるため禁止です。
11. このドキュメントを全てのフェーズ前に鑑として再読し基本的な思想や概念的齟齬が存在しないかを確認します。
12. ルール無視はルールを無視した時点からやり直すこと。

---

# 1. テスト概要

## 1.1 テスト対象

| 修正ID | 内容 | 優先度 |
|--------|------|--------|
| FIX-001 | ffmpegプロセスkill実装 | P0（致命的） |
| FIX-002 | フロントエンド/バックエンド境界修正 | P1（重要） |
| FIX-003 | 巡回終了時クリーンアップ | P2（改善） |

## 1.2 テスト環境

| 項目 | 値 |
|------|-----|
| サーバー | 192.168.125.246 (is22) |
| OS | Linux (OrangePi) |
| タイムゾーン | UTC（ログはJST変換で確認） |
| ブラウザ | Chrome（最新版） |
| ダッシュボードURL | http://192.168.125.246:3000 |

---

# 2. FIX-001 テスト計画

## 2.1 受け入れ基準

| ID | 基準 |
|----|------|
| AC-001-1 | タイムアウト発生時にffmpegプロセスが終了すること |
| AC-001-2 | 正常終了時にJPEG画像が返却されること |
| AC-001-3 | 5分間のポーリング後、ffmpegプロセス数 < 5 |
| AC-001-4 | 既存のCameraGrid表示に影響しないこと |

## 2.2 テストケース

### TC-001-1: 正常なスナップショット取得

```bash
# 事前確認: ffmpegプロセス数
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "ps aux | grep ffmpeg | grep -v grep | wc -l"

# is22サービス再起動
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "echo 'mijeos12345@' | sudo -S systemctl restart is22"

# 30秒待機
sleep 30

# ffmpegプロセス数確認（正常動作中は少数）
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "ps aux | grep ffmpeg | grep -v grep | wc -l"
```

**期待結果**: ffmpegプロセス数が10以下

### TC-001-2: タイムアウト後のプロセス終了

```bash
# 5分間プロセス監視
for i in {1..30}; do
  echo "$(date): $(SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 'ps aux | grep ffmpeg | grep -v grep | wc -l') processes"
  sleep 10
done
```

**期待結果**: 5分後もプロセス数が5以下

### TC-001-3: 応答しないカメラへの対応

```bash
# is22ログでタイムアウトを確認
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "journalctl -u is22 --since '10 minutes ago' --no-pager | grep -E 'timeout.*killing|ffmpeg timeout'"
```

**期待結果**: タイムアウト時に「killing child process」ログが出力

### TC-001-4: CameraGrid表示確認

1. Chromeでhttp://192.168.125.246:3000を開く
2. CameraGridのタイル更新を確認（スナップショットが更新される）
3. 5分間監視し、表示が正常であることを確認

**期待結果**: カメラタイルが正常に更新される

---

# 3. FIX-002 テスト計画

## 3.1 受け入れ基準

| ID | 基準 |
|----|------|
| AC-002-1 | ページリロード時にSuggestPaneでビデオ再生が開始されないこと |
| AC-002-2 | WebSocketイベントでSuggestPaneのビデオ再生が開始されること |
| AC-002-3 | EventLogPaneは過去ログを正常に表示すること |
| AC-002-4 | severity=0イベントは再生トリガーにならないこと |

## 3.2 テストケース

### TC-002-1: ページリロード時の動作

1. Chromeでhttp://192.168.125.246:3000を開く
2. EventLogPaneに過去のイベントが表示されることを確認
3. SuggestPaneを確認（「イベントなし」表示であること）
4. F5でページリロード
5. SuggestPaneが「イベントなし」のままであることを確認

**期待結果**: リロード後もSuggestPaneは「イベントなし」

### TC-002-2: WebSocketイベントでの再生開始

1. Chromeでhttp://192.168.125.246:3000を開く
2. 検知イベントの発生を待つ（または手動トリガー）
3. SuggestPaneでビデオ再生が開始されることを確認
4. カメラ名、LIVEバッジ、検知バッジが表示されることを確認

**期待結果**: リアルタイムイベントでビデオ再生が開始

### TC-002-3: EventLogPane表示確認

1. Chromeでhttp://192.168.125.246:3000を開く
2. EventLogPaneにイベント一覧が表示されることを確認
3. 検知イベント発生後、EventLogPaneに追加されることを確認

**期待結果**: 過去ログと新規イベントが正常に表示

### TC-002-4: DevTools確認（WebSocket）

1. Chrome DevToolsを開く（F12）
2. Networkタブ → WS でWebSocket接続を確認
3. イベント発生時にメッセージが受信されることを確認

**期待結果**: WebSocketメッセージにlog_id, lacis_id, primary_event等が含まれる

---

# 4. FIX-003 テスト計画

## 4.1 受け入れ基準

| ID | 基準 |
|----|------|
| AC-003-1 | サイクル終了後に視聴者なしストリームが削除されること |
| AC-003-2 | 視聴者ありストリームが保護されること |
| AC-003-3 | SuggestPane再生中にサイクル完了しても再生が継続すること |

## 4.2 テストケース

### TC-003-1: go2rtcストリームのクリーンアップ

```bash
# サイクル完了前のストリーム数
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "curl -s 'http://localhost:1984/api/streams' | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))'"

# サイクル完了を待つ（ログで確認）
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "journalctl -u is22 -f --no-pager | grep -m1 'Subnet cycle completed'"

# サイクル完了後のストリーム数
SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 \
  "curl -s 'http://localhost:1984/api/streams' | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))'"
```

**期待結果**: 視聴者なしストリームが減少

### TC-003-2: SuggestPane再生中のストリーム保護

1. Chromeでダッシュボードを開く
2. 検知イベントを待ちSuggestPaneで再生開始
3. サイクル完了を待つ（ログで確認）
4. SuggestPaneの再生が継続していることを確認

**期待結果**: 再生が中断されない

---

# 5. 統合テストシナリオ

## 5.1 シナリオ1: 長時間安定性テスト

**目的**: 1時間連続稼働でffmpegゾンビが蓄積しないことを確認

**手順**:
1. is22サービス再起動
2. 初期ffmpegプロセス数を記録
3. 10分ごとにプロセス数を記録（6回）
4. 最終プロセス数が10以下であることを確認

**実行スクリプト**:

```bash
#!/bin/bash
echo "=== 長時間安定性テスト開始 ==="
echo "開始時刻: $(date)"

# 初期状態
INITIAL=$(SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 'ps aux | grep ffmpeg | grep -v grep | wc -l')
echo "初期プロセス数: $INITIAL"

# 1時間監視
for i in {1..6}; do
  sleep 600
  COUNT=$(SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 'ps aux | grep ffmpeg | grep -v grep | wc -l')
  echo "$(date): $COUNT processes"
done

FINAL=$(SSHPASS='mijeos12345@' sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@192.168.125.246 'ps aux | grep ffmpeg | grep -v grep | wc -l')
echo "最終プロセス数: $FINAL"

if [ "$FINAL" -le 10 ]; then
  echo "✅ テスト合格: プロセス数が10以下"
else
  echo "❌ テスト失敗: プロセス数が10を超過"
fi
```

**合格基準**: 最終プロセス数 ≤ 10

## 5.2 シナリオ2: Chrome UI統合テスト

**目的**: ブラウザでの総合動作確認

**手順**:
1. Chromeでダッシュボードを開く
2. CameraGridが正常に表示されることを確認
3. ページをリロード（F5）
4. SuggestPaneが「イベントなし」であることを確認
5. 検知イベントを待つ
6. SuggestPaneでビデオ再生が開始されることを確認
7. EventLogPaneにイベントが追加されることを確認
8. 10分間放置し、表示が安定していることを確認

**合格基準**: 全ステップで期待通りの動作

---

# 6. テスト実行チェックリスト

## 6.1 FIX-001

- [ ] TC-001-1: 正常なスナップショット取得
- [ ] TC-001-2: タイムアウト後のプロセス終了
- [ ] TC-001-3: 応答しないカメラへの対応
- [ ] TC-001-4: CameraGrid表示確認
- [ ] AC-001-1〜AC-001-4 すべて合格

## 6.2 FIX-002

- [ ] TC-002-1: ページリロード時の動作
- [ ] TC-002-2: WebSocketイベントでの再生開始
- [ ] TC-002-3: EventLogPane表示確認
- [ ] TC-002-4: DevTools確認
- [ ] AC-002-1〜AC-002-4 すべて合格

## 6.3 FIX-003

- [ ] TC-003-1: go2rtcストリームのクリーンアップ
- [ ] TC-003-2: SuggestPane再生中のストリーム保護
- [ ] AC-003-1〜AC-003-3 すべて合格

## 6.4 統合テスト

- [ ] シナリオ1: 長時間安定性テスト（1時間）
- [ ] シナリオ2: Chrome UI統合テスト

---

# 7. テスト結果報告テンプレート

```markdown
# テスト結果報告

**実施日時**: YYYY-MM-DD HH:MM JST
**実施者**:
**対象ビルド**: git commit hash

## FIX-001 結果

| 基準 | 結果 | 備考 |
|------|------|------|
| AC-001-1 | ✅/❌ | |
| AC-001-2 | ✅/❌ | |
| AC-001-3 | ✅/❌ | 最終プロセス数: N |
| AC-001-4 | ✅/❌ | |

## FIX-002 結果

| 基準 | 結果 | 備考 |
|------|------|------|
| AC-002-1 | ✅/❌ | |
| AC-002-2 | ✅/❌ | |
| AC-002-3 | ✅/❌ | |
| AC-002-4 | ✅/❌ | |

## FIX-003 結果

| 基準 | 結果 | 備考 |
|------|------|------|
| AC-003-1 | ✅/❌ | |
| AC-003-2 | ✅/❌ | |
| AC-003-3 | ✅/❌ | |

## 統合テスト結果

- シナリオ1: ✅/❌
- シナリオ2: ✅/❌

## 総合判定

**合格/不合格**

## 備考

（問題点、懸念事項があれば記載）
```

---

# 更新履歴

| 日時 (JST) | 内容 |
|------------|------|
| 2026-01-05 12:45 | 初版作成 |

---

**作成者**: Claude Code
**ステータス**: テスト待ち
