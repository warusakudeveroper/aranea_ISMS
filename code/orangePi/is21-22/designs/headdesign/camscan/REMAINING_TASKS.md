# Camscan機能改善 残タスクリスト

## 必須参照ドキュメント

全タスク実施前に以下を必ず確認すること:

1. **The_golden_rules.md**: `/Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/aranea_ISMS/The_golden_rules.md`
2. **Camscan_designers_review.md**: `./Camscan_designers_review.md` (原典)
3. **TASK_INDEX.md**: `./TASK_INDEX.md` (タスク定義)

---

## 残タスク概要

| 優先度 | カテゴリ | タスク数 | 説明 |
|--------|---------|---------|------|
| P0 | バックエンド必須 | 4 | ハードコード削除・バックエンド実行 |
| P1 | フロントエンド必須 | 5 | UI/UX改善・表示機能 |
| P2 | テスト必須 | 3 | Chrome実UI・単体テスト |

---

## P0: バックエンド必須タスク

### RT-01: oui_data.rs ハードコード削除
**対応設計**: TASK_INDEX.md T4-1, Camscan_designers_review.md Item 1

**現状**:
- `src/ipcam_scan/scanner/oui_data.rs` に OUI_CAMERA_VENDORS がハードコード
- DBのoui_entriesテーブルから読み込む実装が必要

**作業内容**:
1. CameraBrandService からOUIデータを取得
2. oui_data.rs のハードコード定数を削除
3. OUI判定ロジックをDB参照に変更

**受け入れ条件**:
- [x] oui_data.rs からハードコード定数が完全削除 ✅ 2026-01-07完了
- [x] DB oui_entries からの動的読み込みが動作 ✅ load_oui_map()実装
- [x] TP-link以外のブランドOUI追加で判定可能 ✅ 017_oui_expansion.sql追加

**実装詳細**:
- `OUI_CAMERA_VENDORS`定数を削除
- `lookup_oui(mac, oui_map)`に署名変更（OuiMap引数追加）
- `IpcamScan::load_oui_map()`でDBから動的読み込み
- `arp_scan_subnet()`と`verify_device()`でOUIマップ使用

---

### RT-02: RtspTemplate ハードコード削除 ✅ 完了
**対応設計**: TASK_INDEX.md T4-2

**現状**:
- RTSPパステンプレートがコード内にハードコード → 既にDBベースで運用中
- DBのrtsp_templatesテーブルから読み込む実装 → CameraBrandServiceで実装済み

**作業内容**:
1. rtsp_templates テーブルからテンプレート取得 → 実装済み
2. ハードコードされたパス定義を削除 → 015_oui_rtsp_ssot.sqlで対応
3. ブランド別RTSPパス解決ロジックをDB参照に変更 → CameraBrandService実装

**受け入れ条件**:
- [x] ハードコードされたRTSPパスが完全削除 ✅ DBベースに移行済み
- [x] DB rtsp_templates からの動的読み込みが動作 ✅ CameraBrandService
- [x] 新規ブランド追加時にRTSPパスが自動適用 ✅ 017_oui_expansion.sqlで検証

---

### RT-03: バックエンド単一実行化 ✅ 完了
**対応設計**: Camscan_designers_review.md Item 3

**現状**:
- スキャンはモーダル閉じると停止する可能性あり → tokio::spawnで解決
- フロントエンド依存の実行モデル → バックエンド独立実行に移行

**作業内容**:
1. スキャンジョブをバックエンドで完全独立実行 ✅ tokio::spawn
2. モーダル閉じてもスキャン継続 ✅ 実装済み
3. ジョブステータスポーリングでUI更新 ✅ GET /api/ipcamscan/jobs/:id
4. 「スキャン中止」ボタンで明示的停止のみ ✅ POST /api/ipcamscan/jobs/:id/abort

**受け入れ条件**:
- [x] モーダル閉じてもスキャン継続を確認 ✅ tokio::spawnでバックグラウンド実行
- [x] バックグラウンドスキャン状態でモーダル再開可能 ✅ GET /api/ipcamscan/status
- [x] 並列スキャン防止（単一実行保証） ✅ running_job_id + create_job()でConflictエラー

**実装詳細**:
- `running_job_id: Arc<RwLock<Option<Uuid>>>` で実行中ジョブを追跡
- `abort_flags: Arc<RwLock<HashMap<Uuid, bool>>>` で中止リクエストを管理
- `create_job()` → `Result<ScanJob>` に変更、既に実行中なら`Error::Conflict`
- `run_job()` → 開始時に`running_job_id`設定、各Stage間で中止チェック
- 新規API: `GET /api/ipcamscan/status` (実行状態確認)
- 新規API: `POST /api/ipcamscan/jobs/:id/abort` (中止リクエスト)

---

### RT-04: OUI拡張（主要ブランド追加） ✅ 完了
**対応設計**: Camscan_designers_review.md Item 1

**追加対象ブランド** (017_oui_expansion.sqlで追加):
1. Anker Eufy ✅
2. Ring ✅ (015で追加済み)
3. SwitchBot ✅ (015で追加済み)
4. EZVIZ ✅ (015で追加済み)
5. Wyze ✅ (新規追加)
6. I.O.data ✅ (015で追加済み)
7. Panasonic ✅ (015で追加済み)
8. Arlo ✅ (015で追加済み)
9. Xiaomi/Mi Home ✅ (新規追加)
10. Reolink ✅ (015で追加済み)
11. Amcrest ✅ (015で追加済み)
12. Hikvision ✅ (015で追加済み)
13. Dahua ✅ (015で追加済み)
14. Amazon Blink ✅ (新規追加)
15. Aqara ✅ (新規追加)
16. Imou ✅ (新規追加)
17. Ubiquiti UniFi ✅ (新規追加)

**作業内容**:
1. 各ブランドのOUIプレフィックスを調査 ✅ maclookup.appで調査
2. camera_brands テーブルにブランド追加 ✅ 7ブランド新規追加
3. oui_entries テーブルにOUI追加 ✅ 60+件追加
4. rtsp_templates テーブルにパステンプレート追加 ✅ 5テンプレート追加

**受け入れ条件**:
- [x] 上記ブランドがDB登録済み ✅ 017_oui_expansion.sql
- [x] OUIスキャンで各ブランド判定可能 ✅ load_oui_map()連動
- [x] RTSPパステンプレートが適用される ✅ CameraBrandService連動

**マイグレーションファイル**: `migrations/017_oui_expansion.sql`

---

## P1: フロントエンド必須タスク

### RT-05: 登録済みカメラ名表示 ✅ 完了
**対応設計**: Camscan_designers_review.md Item 2, 6

**現状**:
- カテゴリAに登録済みカメラのIP表示のみ → カメラ名表示を追加
- カメラ名（表示名）が表示されない → 太字で目立つように表示

**作業内容**:
1. スキャン結果にregistered_camera_name取得 ✅
2. カテゴリAデバイスカードにカメラ名を太字で表示 ✅
3. カテゴリFにも登録カメラ名を表示（既存だが確認） ✅

**受け入れ条件**:
- [x] カテゴリAでカメラ名が太字で表示 ✅
- [x] カテゴリFでカメラ名が表示（通信断時） ✅

**実装詳細**:
- `ScanModal.tsx`: `isRegisteredCamera`判定追加
- `CategorizedDevice`に`registered_camera_name`表示

---

### RT-06: 過去スキャン結果タイムスタンプ表示 ✅ 完了
**対応設計**: Camscan_designers_review.md Item 2

**現状**:
- 過去のスキャン結果が表示されるが日時不明 → last_seen表示追加
- 「今回スキャンしたサブネットじゃない」という混乱 → タイムスタンプで明示

**作業内容**:
1. 各デバイスのlast_seen/first_seenを表示 ✅
2. 過去結果には「最終検出: YYYY/MM/DD HH:mm」表示 ✅
3. 今回スキャン対象外サブネットにラベル追加 ✅

**受け入れ条件**:
- [x] 各デバイスにスキャン日時表示 ✅
- [x] 今回スキャン対象外が明示される ✅

**実装詳細**:
- `ScanModal.tsx`: `last_seen`をja-JP形式で表示
- 日時フォーマット: `YYYY/MM/DD HH:mm`

---

### RT-07: 強制登録機能 ✅ 完了
**対応設計**: Camscan_designers_review.md Item 10, TASK_INDEX.md T2-10

**現状**:
- カテゴリCのカメラが選択できても登録時に弾かれる → 選択可能に変更
- 「カメラだから登録しておきたい」ができない → 強制登録機能追加

**作業内容**:
1. カテゴリCデバイスの選択・登録を許可 ✅
2. 登録時にPendingAuthステータスで保存 ✅
3. 認証成功後にActiveに遷移 ✅
4. UIに「強制登録」ボタン/オプション追加 ✅

**受け入れ条件**:
- [x] カテゴリCデバイスが登録可能 ✅
- [x] PendingAuthステータスで登録される ✅
- [x] 認証後にActive遷移 ✅

**実装詳細**:
- `selectAllIncludingPending()`: カテゴリB+Cをまとめて選択
- `handleRegister()`: `force_register`, `initial_status: 'pending_auth'`送信
- バックエンド: `ForceRegisterRequest`対応

---

### RT-08: サブネット削除時クリーンアップ ✅ 完了
**対応設計**: Camscan_designers_review.md Item 11

**現状**:
- サブネット削除してもスキャン結果に残る → 連動削除オプション追加
- 削除したサブネットのカメラが登録可能として表示 → クリーンアップ

**作業内容**:
1. サブネット削除APIでスキャン結果も削除 ✅
2. 登録済みカメラは削除対象外（ユーザー手動削除） ✅
3. 確認ダイアログで影響範囲表示 ✅

**受け入れ条件**:
- [x] サブネット削除でスキャン結果連動削除 ✅
- [x] 登録済みカメラは維持 ✅
- [x] 削除確認UIあり ✅

**実装詳細**:
- `DELETE /api/subnets/:id?cleanup_scan_results=true`
- `DeleteSubnetQuery`でクエリパラメータ解析
- `ipcamscan_devices`からsubnet一致レコードを削除
- フロントエンド: `confirmDeleteSubnet()`, `executeDeleteSubnet()`

---

### RT-09: スキャン結果文言改善 ✅ 完了
**対応設計**: Camscan_designers_review.md Item 8

**現状**:
- 技術的な文言が多い → ユーザーフレンドリーに改善
- ユーザー視点での説明不足 → 次のアクション提案追加

**作業内容**:
1. カテゴリ説明文を平易に ✅
2. 次のアクション提案を追加 ✅
3. エラー・警告メッセージの改善 ✅

**受け入れ条件**:
- [x] 非技術者にわかりやすい文言 ✅
- [x] 次のアクションが明確 ✅

**実装詳細**:
- `DEVICE_CATEGORIES`の`label`/`description`/`userAction`を改善
- カテゴリ表示に説明文と推奨アクションを表示
- 例: 「登録済み」→「登録済みカメラ」、「認証必要」→「認証情報の設定が必要」

---

## P2: テスト必須タスク

### RT-10: Chrome実UIテスト
**対応設計**: TASK_INDEX.md T4-5, The_golden_rules.md Rule 5

**テストケース**:
1. 設定モーダル→カメラブランドタブ切り替え
2. ブランド追加→スキャン→OUI判定
3. Brute Force Modeトグル動作
4. スキャン中止ボタン動作
5. PhaseIndicator進捗連動
6. カテゴリ別表示確認
7. 強制登録フロー

**受け入れ条件**:
- [ ] 全テストケースがChrome実UIで確認
- [ ] スクリーンショット証跡

---

### RT-11: バックエンド単体テスト拡充 ✅ 完了
**対応設計**: TASK_INDEX.md T4-3

**テスト対象**:
1. CameraBrandService CRUD ✅
2. OUI検索性能（<100ms） ✅
3. キャッシュリフレッシュ ✅
4. 進捗計算精度 ✅
5. LostConnection注入 ✅

**受け入れ条件**:
- [x] 各サービスの単体テスト追加 ✅ 28テスト
- [x] cargo test 全パス ✅ 2026-01-07確認

---

### RT-12: マイグレーションロールバック検証
**対応設計**: TASK_INDEX.md T4-6

**検証内容**:
1. camera_brands, oui_entries, rtsp_templates削除
2. 既存カメラデータへの影響なし確認
3. ロールバック後の再マイグレーション

**受け入れ条件**:
- [ ] ロールバック手順文書化
- [ ] ロールバック・再適用テスト完了

---

## GitHub Issue構成

| Issue # | タイトル | 含むタスク | URL |
|---------|---------|-----------|-----|
| #87 | [Camscan] ハードコード削除・OUI拡張 | RT-01, RT-02, RT-04 | https://github.com/warusakudeveroper/aranea_ISMS/issues/87 |
| #88 | [Camscan] バックエンド単一実行化 | RT-03 | https://github.com/warusakudeveroper/aranea_ISMS/issues/88 |
| #89 | [Camscan] フロントエンドUI改善 | RT-05, RT-06, RT-07, RT-08, RT-09 | https://github.com/warusakudeveroper/aranea_ISMS/issues/89 |
| #90 | [Camscan] 完了テスト・検証 | RT-10, RT-11, RT-12 | https://github.com/warusakudeveroper/aranea_ISMS/issues/90 |

---

## 依存関係

```
RT-01 (OUIハードコード削除)
  └── RT-04 (OUI拡張) ※同時実施推奨

RT-02 (RTSPハードコード削除)
  └── RT-04 (OUI拡張) ※ブランド連動

RT-03 (バックエンド単一実行) ※独立

RT-05〜RT-09 (フロントエンド) ※バックエンド完了後

RT-10〜RT-12 (テスト) ※全実装完了後
```

---

## MECE確認

- [x] Camscan_designers_review.md 11項目すべてカバー
- [x] TASK_INDEX.md Phase 4タスクすべてカバー
- [x] The_golden_rules.md要件（テスト必須）対応
- [x] 依存関係が明確
- [x] 各タスクに受け入れ条件

---

**作成日**: 2026-01-07
**最終更新**: 2026-01-07
- Issue #87完了: RT-01, RT-02, RT-04 (ハードコード削除・OUI拡張)
- Issue #88完了: RT-03 (バックエンド単一実行化)
- Issue #89完了: RT-05〜RT-09 (フロントエンドUI改善)
- Issue #90進行中: RT-10〜RT-12 (テスト・検証) - バックエンド28テスト・フロントエンド47テスト通過
