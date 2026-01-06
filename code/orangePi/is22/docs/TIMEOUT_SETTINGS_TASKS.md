# タイムアウト設定機能 タスクリスト・テスト計画

**親ドキュメント**:
- [TIMEOUT_SETTINGS_DESIGN.md](./TIMEOUT_SETTINGS_DESIGN.md)
- [TIMEOUT_SETTINGS_DETAILED_DESIGN.md](./TIMEOUT_SETTINGS_DETAILED_DESIGN.md)

**作成日**: 2026-01-06

---

## Phase 1: グローバルタイムアウト設定

### Phase 1.1: Backend API実装

#### Task 1.1.1: GET /api/settings/timeouts 実装
**担当モジュール**: `src/web_api/routes.rs`

**実装内容**:
- [ ] `get_global_timeouts`関数を実装
- [ ] settings.pollingからtimeout_main_sec/timeout_sub_secを取得
- [ ] デフォルト値(10/20)のフォールバック処理
- [ ] JSON形式でレスポンス返却

**受け入れ基準**:
- [ ] curl テスト: `curl http://localhost:8080/api/settings/timeouts`
- [ ] レスポンス形式が正しいか確認
- [ ] settings.pollingが存在しない場合のフォールバックが動作

**推定時間**: 30分

---

#### Task 1.1.2: PUT /api/settings/timeouts 実装
**担当モジュール**: `src/web_api/routes.rs`

**実装内容**:
- [ ] `UpdateTimeoutRequest`構造体を定義
- [ ] `update_global_timeouts`関数を実装
- [ ] バリデーション: 5 <= value <= 120
- [ ] settings.pollingをJSON_SETで更新
- [ ] ConfigStore.refresh_cache()を呼び出し

**受け入れ基準**:
- [ ] curl テスト: `curl -X PUT -H "Content-Type: application/json" -d '{"timeout_main_sec":15,"timeout_sub_sec":30}' http://localhost:8080/api/settings/timeouts`
- [ ] DBに値が保存されているか確認
- [ ] 範囲外の値でエラーが返るか確認（4秒, 121秒）

**推定時間**: 45分

---

#### Task 1.1.3: ルート登録
**担当モジュール**: `src/web_api/routes.rs`

**実装内容**:
- [ ] `build_router()`内に以下を追加:
  ```rust
  .route("/api/settings/timeouts", get(get_global_timeouts))
  .route("/api/settings/timeouts", put(update_global_timeouts))
  ```

**受け入れ基準**:
- [ ] サーバー起動時にエラーが出ないこと
- [ ] curlでAPIにアクセスできること

**推定時間**: 5分

---

### Phase 1.2: SnapshotService修正

#### Task 1.2.1: コンストラクタ修正
**担当モジュール**: `src/snapshot_service/mod.rs`

**実装内容**:
- [ ] `new()`にtimeout_main_sec, timeout_sub_secパラメータ追加
- [ ] 既存の呼び出し元（PollingOrchestrator）を修正

**受け入れ基準**:
- [ ] コンパイルエラーがないこと
- [ ] 既存の動作に影響がないこと

**推定時間**: 15分

---

### Phase 1.3: PollingOrchestrator修正

#### Task 1.3.1: 設定読み込みロジック実装
**担当モジュール**: `src/polling_orchestrator/mod.rs`

**実装内容**:
- [ ] `GlobalTimeoutSettings`構造体を定義
- [ ] `load_global_timeout_settings()`関数を実装
- [ ] `spawn_subnet_loop`内でSnapshotService初期化時に設定を注入

**受け入れ基準**:
- [ ] ログに`Loaded global timeout settings`が出力される
- [ ] タイムアウト値が正しく読み込まれている

**推定時間**: 30分

---

### Phase 1.4: Frontend実装

#### Task 1.4.1: SettingsModal UI追加
**担当モジュール**: `frontend/src/components/SettingsModal.tsx`

**実装内容**:
- [ ] 状態管理（useState）追加: timeoutMainSec, timeoutSubSec, savingTimeouts
- [ ] UIコンポーネント追加（Cardコンポーネント）
- [ ] Input要素（type="number", min=5, max=120）
- [ ] 保存ボタン

**受け入れ基準**:
- [ ] ブラウザで表示が崩れていないこと
- [ ] 入力欄に数値が入力できること
- [ ] 範囲外の値が入力できないこと

**推定時間**: 45分

---

#### Task 1.4.2: API連携実装
**担当モジュール**: `frontend/src/components/SettingsModal.tsx`

**実装内容**:
- [ ] `fetchTimeoutSettings()`関数を実装
- [ ] `handleSaveTimeouts()`関数を実装
- [ ] useEffectで「表示」タブ表示時にfetch

**受け入れ基準**:
- [ ] モーダルを開くと現在の値が表示される
- [ ] 値を変更して保存ボタンをクリックするとAPIが呼ばれる
- [ ] 保存後、再度開くと新しい値が表示される

**推定時間**: 30分

---

### Phase 1.5: ビルド・デプロイ・テスト

#### Task 1.5.1: Backend ビルド
**実施内容**:
- [ ] `cargo build --release`を実行
- [ ] コンパイルエラーがないこと確認

**推定時間**: 5分（ビルド時間含む）

---

#### Task 1.5.2: Frontend ビルド
**実施内容**:
- [ ] `npm run build`を実行
- [ ] ビルドエラーがないこと確認

**推定時間**: 2分

---

#### Task 1.5.3: デプロイ
**実施内容**:
- [ ] サーバーへバイナリ転送
- [ ] camserverサービス再起動
- [ ] フロントエンドファイル配置

**推定時間**: 5分

---

#### Task 1.5.4: 統合テスト実行
**テストケース**: Test Case 1（グローバル設定の動作確認）

**手順**:
1. [ ] Chromeでis22 UIを開く
2. [ ] SettingsModalを開き、「表示」タブに移動
3. [ ] タイムアウト値を15秒/30秒に変更
4. [ ] 保存ボタンをクリック
5. [ ] 次の巡回サイクルを待つ（約1分）
6. [ ] サーバーログで`timeout_main_sec=15, timeout_sub_sec=30`を確認

**期待結果**:
- [ ] UI操作がスムーズに動作
- [ ] 保存後、DB に値が保存されている
- [ ] ログに新しいタイムアウト値が出力される
- [ ] 全カメラが新しいタイムアウトで動作

**推定時間**: 15分

---

**Phase 1 合計推定時間**: 約3時間10分

---

## Phase 2: カメラ別カスタムタイムアウト設定

### Phase 2.1: データベースマイグレーション

#### Task 2.1.1: マイグレーションファイル作成
**実施内容**:
- [ ] `015_camera_custom_timeouts.sql`を作成
- [ ] ALTER TABLE文を記述
- [ ] INDEXを追加

**配置場所**: `/opt/is22/migrations/015_camera_custom_timeouts.sql`

**受け入れ基準**:
- [ ] SQLファイルが正しく配置されている
- [ ] 構文エラーがないこと

**推定時間**: 10分

---

#### Task 2.1.2: マイグレーション実行
**実施内容**:
- [ ] サーバー上でマイグレーションを実行
- [ ] 実行ログを確認
- [ ] カラム追加を確認: `DESCRIBE cameras;`

**受け入れ基準**:
- [ ] custom_timeout_enabled, timeout_main_sec, timeout_sub_secカラムが追加されている
- [ ] 既存データに影響がない（デフォルト値0/NULL）
- [ ] INDEXが作成されている

**推定時間**: 5分

---

### Phase 2.2: Backend修正

#### Task 2.2.1: Camera構造体拡張
**担当モジュール**: `src/config_store/types.rs` または `src/models.rs`

**実施内容**:
- [ ] Cameraフィールドを追加:
  - custom_timeout_enabled: bool
  - timeout_main_sec: Option<u64>
  - timeout_sub_sec: Option<u64>

**受け入れ基準**:
- [ ] コンパイルエラーがないこと

**推定時間**: 5分

---

#### Task 2.2.2: ConfigStore修正
**担当モジュール**: `src/config_store/repository.rs`

**実施内容**:
- [ ] `load_cameras_from_db`のSELECT文に新しいカラムを追加

**受け入れ基準**:
- [ ] カメラ読み込み時にエラーが出ないこと
- [ ] 新しいフィールドが正しく読み込まれる

**推定時間**: 10分

---

#### Task 2.2.3: SnapshotService修正（タイムアウト適用ロジック）
**担当モジュール**: `src/snapshot_service/mod.rs`

**実施内容**:
- [ ] `capture()`メソッドの先頭でタイムアウト値を決定:
  ```rust
  let timeout_main = if camera.custom_timeout_enabled {
      camera.timeout_main_sec.unwrap_or(self.ffmpeg_timeout_main)
  } else {
      self.ffmpeg_timeout_main
  };
  ```
- [ ] `capture_rtsp`呼び出し時に動的なタイムアウト値を渡す
- [ ] ログ出力を追加（debug レベル）

**受け入れ基準**:
- [ ] カスタムタイムアウト有効なカメラで個別値が使用される
- [ ] カスタムタイムアウト無効なカメラでグローバル値が使用される
- [ ] ログに適用されたタイムアウト値が出力される

**推定時間**: 30分

---

#### Task 2.2.4: Camera更新API拡張
**担当モジュール**: `src/web_api/routes.rs`

**実施内容**:
- [ ] `UpdateCameraRequest`に新しいフィールドを追加
- [ ] UPDATE クエリに新しいカラムを追加
- [ ] バリデーション追加（5 <= value <= 120）

**受け入れ基準**:
- [ ] curl テスト: カメラ更新APIで新しいフィールドが保存される
- [ ] 範囲外の値でエラーが返る

**推定時間**: 30分

---

### Phase 2.3: Frontend修正

#### Task 2.3.1: types/api.ts 修正
**担当モジュール**: `frontend/src/types/api.ts`

**実施内容**:
- [ ] Camera型に新しいフィールドを追加

**受け入れ基準**:
- [ ] TypeScriptコンパイルエラーがないこと

**推定時間**: 2分

---

#### Task 2.3.2: CameraDetailModal UI追加
**担当モジュール**: `frontend/src/components/CameraDetailModal.tsx`

**実施内容**:
- [ ] 状態管理（useState）追加: customTimeoutEnabled, timeoutMainSec, timeoutSubSec
- [ ] チェックボックス追加（「カスタムタイムアウトを使用」）
- [ ] Input要素（type="number", min=5, max=120）×2
- [ ] チェックボックスON時のみ入力欄を有効化

**受け入れ基準**:
- [ ] チェックボックスで入力欄の有効/無効が切り替わる
- [ ] 範囲外の値が入力できない

**推定時間**: 45分

---

#### Task 2.3.3: CameraDetailModal API連携
**担当モジュール**: `frontend/src/components/CameraDetailModal.tsx`

**実施内容**:
- [ ] useEffectで初期値をセット
- [ ] handleSave関数にペイロード追加

**受け入れ基準**:
- [ ] モーダルを開くと現在の値が表示される
- [ ] 保存後、DBに値が保存される
- [ ] 再度開くと保存した値が表示される

**推定時間**: 15分

---

### Phase 2.4: ビルド・デプロイ・テスト

#### Task 2.4.1: Backend ビルド
**実施内容**:
- [ ] `cargo build --release`を実行
- [ ] コンパイルエラーがないこと確認

**推定時間**: 5分

---

#### Task 2.4.2: Frontend ビルド
**実施内容**:
- [ ] `npm run build`を実行
- [ ] ビルドエラーがないこと確認

**推定時間**: 2分

---

#### Task 2.4.3: デプロイ
**実施内容**:
- [ ] サーバーへバイナリ転送
- [ ] camserverサービス再起動
- [ ] フロントエンドファイル配置

**推定時間**: 5分

---

#### Task 2.4.4: 統合テスト実行
**テストケース**: Test Case 2（カメラ個別設定の優先）

**手順**:
1. [ ] グローバル設定を10秒/20秒に設定
2. [ ] カメラA（.62カメラ）の詳細モーダルを開く
3. [ ] 「カスタムタイムアウトを使用」をチェック
4. [ ] タイムアウトを30秒/60秒に設定して保存
5. [ ] カメラB（別のカメラ）はカスタムOFF
6. [ ] 次の巡回サイクルを待つ
7. [ ] ログでカメラA/Bのタイムアウト値を確認

**期待結果**:
- [ ] カメラA: `timeout_main_sec=30, timeout_sub_sec=60`を使用
- [ ] カメラB: `timeout_main_sec=10, timeout_sub_sec=20`を使用（グローバル）
- [ ] カメラAの成功率が改善

**推定時間**: 20分

---

**Phase 2 合計推定時間**: 約2時間30分

---

## Phase 3: 統合テスト・最終検証

### Phase 3.1: フォールバック動作確認

#### Task 3.1.1: Test Case 3（フォールバック動作）
**手順**:
1. [ ] カメラCをDBで直接操作: `custom_timeout_enabled=1, timeout_main_sec=NULL, timeout_sub_sec=NULL`
2. [ ] 次の巡回サイクルを待つ
3. [ ] ログを確認

**期待結果**:
- [ ] グローバル設定にフォールバックする
- [ ] ログに警告が出力される（データ不整合の検知）

**推定時間**: 10分

---

### Phase 3.2: 実カメラ検証

#### Task 3.2.1: Test Case 4（.62カメラでの実証）
**手順**:
1. [ ] .62カメラの現在の成功率を記録（3-5サイクル分）
2. [ ] カスタムタイムアウトを30秒/40秒に設定
3. [ ] 5サイクル実行
4. [ ] 成功率を比較

**期待結果**:
- [ ] タイムアウト延長前: 0-50%成功率
- [ ] タイムアウト延長後: 80-100%成功率
- [ ] 成功時のスナップショット画像が正常に保存される

**推定時間**: 30分（巡回待ち時間含む）

---

### Phase 3.3: パフォーマンステスト

#### Task 3.3.1: Test Case 5（巡回時間への影響測定）
**手順**:
1. [ ] 125サブネット全カメラを10秒/20秒に設定
2. [ ] 3サイクル実行し、平均巡回時間を記録
3. [ ] 125サブネット全カメラを30秒/60秒に設定
4. [ ] 3サイクル実行し、平均巡回時間を記録
5. [ ] CPU/メモリ使用率を確認

**期待結果**:
- [ ] 巡回時間が適切に増加（線形的）
- [ ] CPU/メモリ使用率に異常な増加がない

**推定時間**: 30分

---

### Phase 3.4: UI/UXテスト

#### Task 3.4.1: Test Case 6（SettingsModal入力検証）
**手順**:
1. [ ] タイムアウトに4秒を入力 → 入力できないor警告表示
2. [ ] タイムアウトに121秒を入力 → 入力できないor警告表示
3. [ ] タイムアウトに文字列を入力 → 入力不可
4. [ ] タイムアウトに50秒を入力 → 保存成功

**期待結果**:
- [ ] 範囲外の値が適切に拒否される
- [ ] エラーメッセージが表示される（必要に応じて）

**推定時間**: 10分

---

#### Task 3.4.2: Test Case 7（CameraDetailModal連携）
**手順**:
1. [ ] カメラ詳細モーダルを開く
2. [ ] 「カスタムタイムアウトを使用」をチェック → 入力欄が有効化
3. [ ] 値を入力して保存
4. [ ] モーダルを閉じて再度開く → 値が保持されている
5. [ ] チェックを外して保存 → DB でcustom_timeout_enabled=0に更新
6. [ ] 再度開く → チェックが外れている

**期待結果**:
- [ ] チェックボックスの状態が正しく保存・復元される
- [ ] 入力欄の有効/無効が正しく切り替わる

**推定時間**: 15分

---

**Phase 3 合計推定時間**: 約1時間35分

---

## 全体タスク総括

### 推定時間合計
- **Phase 1**: 3時間10分
- **Phase 2**: 2時間30分
- **Phase 3**: 1時間35分
- **合計**: **約7時間15分**

### 依存関係フローチャート

```
Phase 1: グローバル設定
  ├─ 1.1 Backend API ────┐
  ├─ 1.2 SnapshotService ├─→ 1.5 ビルド・デプロイ・テスト
  ├─ 1.3 Orchestrator ───┤
  └─ 1.4 Frontend ───────┘
                          ↓
Phase 2: カメラ別設定 (Phase 1完了必須)
  ├─ 2.1 DB Migration ───┐
  ├─ 2.2 Backend ────────├─→ 2.4 ビルド・デプロイ・テスト
  └─ 2.3 Frontend ───────┘
                          ↓
Phase 3: 統合テスト (Phase 2完了必須)
  ├─ 3.1 フォールバック
  ├─ 3.2 実カメラ検証
  ├─ 3.3 パフォーマンス
  └─ 3.4 UI/UX
```

### 実施順序の大原則（The_golden_rules.md #7）

**優先順位は依存関係と実装順によってのみ定義される。**

1. Phase 1を完全に完了させるまでPhase 2に進まない
2. Phase 2を完全に完了させるまでPhase 3に進まない
3. 各Phase内でも依存関係に従い順序を遵守

### 完了判定基準

以下のすべてを満たした場合に「完了」とする：

- [ ] すべてのタスクが✅チェック済み
- [ ] すべてのテストケースがパス
- [ ] 実カメラ（.62）で成功率改善が確認された
- [ ] パフォーマンス劣化がない
- [ ] The_golden_rules.mdの原則に違反がない
- [ ] コードレビュー完了
- [ ] ドキュメント更新完了

---

**文書ステータス**: タスクリスト・テスト計画完成
**次のアクション**: GitHub Issue登録 → 実装前コミット → Phase 1実装開始
