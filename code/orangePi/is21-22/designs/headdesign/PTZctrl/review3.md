了解。コミット **b3fae198**（`fix(is22): レビュー指摘事項対応 - Blocker/High優先度修正`）時点のコードを、こちらのレビュー指摘フォーマットに沿って「修正が本当に効いているか」「副作用が出てないか」を再レビューしました。

---

## 1) Blocker 修正の再レビュー結果

### ✅ target/ がコミットされている

* コミット差分上は `code/orangePi/is22/tests/e2e_paraclate/target/**` が大量削除されており（target由来のファイル群が消えている）、現行ツリー検索でも `e2e_paraclate/target` はヒットしませんでした（※履歴圧縮が必要なら別途 filter-repo 系が要ります）。

### ✅ テストにDBパスワード直書き

* `test_camera_registration.sh` は **`CAMSERVER_DB_PASS` を必須環境変数として要求**する形に変更されています（`DB_PASS="${CAMSERVER_DB_PASS:?...}"`）
* `.env.example` が追加され、DB/SSH の環境変数がテンプレ化されています
* `tests/README.md` に **`.env.example → .env → source`** の手順と必須環境変数が明記されています

→ **このBlockerは解消**です。

### ⚠️ Paraclate型が不一致（「型」だけは直ったが、実装が追随していない）

* `frontend/src/types/paraclate.ts` はバックエンド準拠に更新されています（`connectionStatus` から `connecting` を削除、`ConnectResponse` を `connected/endpoint/configId/error` に統一、`QueueStatus` に `skipped` 追加、`stats` 化など）
* しかし `frontend/src/hooks/useParaclateStatus.ts` 側は **まだ旧仕様のまま**で、`connect()` が `data.success` を見て分岐し、失敗時に `message` を扱い、catchで `{ success, connected, message }` を返しています
  これは新しい `ParaclateConnectResponse`（`success/message` が存在しない） と矛盾します。
* バックエンドの `ConnectResponse` は `camelCase` で **`connected/endpoint/configId/error`** が正です

→ ここは **実行時だけでなくTSビルド段階でも破綻しうる**ので、現状は「Blockerが別の形で残っている」と判断します。

**最短修正案（hook側）**

* `connect()`：

  * `if (data.connected && !data.error) { await refetch(); } else { setError(data.error ?? 'Connection failed'); }`
  * catch return は `{ connected:false, endpoint, configId:0, error:errorMessage }` みたいに **新型に合わせる**
* `testConnection()`：今は `Endpoint not configured` を `{success,message}` で返しているので、新型（`endpoint/configId/error`）に合わせて返す

※ `disconnect` API はバックエンドが `{success:true,message:"..."}` を返す仕様なので、disconnect側の型は別（汎用 `{success:boolean; message?:string; error?:string}`）に分けるのが一番安全です。

---

## 2) High 修正の再レビュー結果

### ✅ PTZ nudge stop が実質No-op

* `PtzService.move_ptz()` の Nudge 分岐で、`duration_ms` 後に `execute_ptz_stop_with_config()` を呼ぶようになっています
* `execute_ptz_stop_with_config()` は `tapo/vigi` の場合に **実際に `client.stop()` を呼ぶ**実装になっています

→ **このHighは解消**です。

### ✅ rtsp_sub をテストが見ていない

* `verify_camera_fields()` に `rtsp_main` と `rtsp_sub` の検証が入り、`tapo/vigi` では未設定を FAIL 扱いにしています

→ **このHighは解消**です。

---

## 3) 残Medium（現状維持でOKだが、次に効く順）

### Medium-1: Paraclate endpoint「保存するが実送信は固定URL」

* これは設計上の一貫性課題のまま（review2.mdでも残件）
  → “base endpoint + path結合” に寄せるか、「固定URL運用」を仕様として明記、のどちらかに寄せるのが良いです。

### Medium-2: AccessAbsorber acquire/preempt の競合耐性

* 方針は良いが、同時実行時の整合（DBトランザクション/ロック粒度）は引き続き要確認（残件のままでOK）

### Medium-3: IpcamScan list_devices の全件取得→Rustフィルタ

* 現状「simple implementation」として残っているので、将来台数が増えたらSQL側 `WHERE` に寄せるのが効果大です

---

## 4) 追加で見つかった“ドキュメント負債”

* DD09（WebUI詳細設計）は旧 `ParaclateConnectResponse(success/message/latencyMs)` のまま残っています
  実装がSSoTに寄ったなら、DD09も合わせて更新しておくと後続が迷いません（特にAIエージェント運用で事故ります）。

---

# まとめ

* **target除去 / テスト秘匿情報 / PTZ stop / rtsp_subテスト**は期待通り改善されています  
* ただし **Paraclateは「型定義はSSoTに統一できたが、hook実装が旧仕様のまま」**で、ここが実質Blockerとして残っています  


