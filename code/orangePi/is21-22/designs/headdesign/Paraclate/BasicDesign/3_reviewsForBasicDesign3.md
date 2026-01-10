# review1
## 再レビュー結果（Issue #111 / commit 515c509）

結論：**Issue #111 に列挙された P0（4件）＋P1（1件）は、いずれも修正反映を確認できました。** 
ただし **「ドキュメント間の整合性」観点で残っている差分が3点**あり、ここは次の事故（実装時の手戻り）に直結しやすいので、**追加で直してから #111 をクローズ**が安全です。

---

# 1) #111 の対応完了確認（P0/P1）

### P0-1: is21 の fid 統一（9966→0000）

`02_is21_status.md` の権限設定が **FID:0000** に修正され、注釈も追加されています。

### P0-2: DesignOverview の Summary JSON を有効化

Summary ペイロードが **有効なJSON**（`"key":"value"`）に修正され、構造もオブジェクト化されています。

### P0-3: TypeDomain を araneaDevice（単数形）に統一

DesignOverview 内の表記が **TypeDomain=araneaDevice / Type=ar-is22CamServer / Prefix=3** に統一されています。

### P0-4: 連携設計の認証方式を lacisOath に統一（JWT廃止）

`05_paraclate_integration.md` で JWT/Bearer を廃止し、**リクエストボディの lacisOath で認証**に統一されていることを確認しました。

### P1-2: 韓国語混入の解消＋SSoT表整形

`Paraclate_FunctionArrangement(AIchat).md` の SSoT表が整形され、「現在」に修正済み、LacisFiles 行も追加されています。 

---

# 2) 追加指摘（整合性レビューで残っている差分）

## (A) 【P0相当】03_authentication.md がまだ “旧表記” のまま（登録実装を誤誘導）

`03_authentication.md` の is22用パラメータ表が **TypeDomain=araneaDevices / Type=ar-is22Camserver / Prefix=3(araneaDevices共通)** のまま残っています。
一方で DesignOverview は **TypeDomain=araneaDevice / Type=ar-is22CamServer** に正規化済みです。

**なぜP0か**：AraneaRegister（Rust実装）で *typeDomain/type* を間違えると、mobes側の typeSettings / default permissions / UIフィルタで破綻しやすく、登録～権限付与がズレます（＝実装での致命傷）。

✅ **修正指示（アンアンビギュアス）**

* `03_authentication.md` の表（2.2）を **DesignOverview と同一値**に置換

  * `TypeDomain: araneaDevice`
  * `Type: ar-is22CamServer`
  * `Prefix: 3（araneaDevice共通）`

---

## (B) 【P1】05_paraclate_integration.md の Summary例が “旧フォーマット” のまま

DesignOverview は Summary JSON を新フォーマット（`cameraContext: object`, `cameraDetection: object[]`, `lastDetectAt`）に更新済みです。
しかし integration 側の例は `fendDetectAt` のまま、`cameraContext` が配列、`cameraDetection` がCSV文字列配列のままです。

✅ **修正指示**

* integration の例を **DesignOverview の送信形式と一致**させる

  * `fendDetectAt` → `lastDetectAt`
  * `cameraContext` → オブジェクト
  * `cameraDetection` → `{timestamp, cameraLacisId, detectionDetail}[]`

---

## (C) 【P1】DesignOverview の lacisOath 例が “JSONとして壊れている”

DesignOverview の `lacisOath` サンプルは ` ```json ` で囲われていますが、現状は **構文が壊れており**、さらに `cic` の値が `fid` と混線しています。

✅ **修正指示**

* ここは「仕様の核」なので、**JSONとして正しい形**に直す（最低限：`"lacisOath": { "lacisID": "...", "tid": "...", "cic": "...", "blessing": "..." }`）

---

# 3) 3_reviewsForBasicDesign.md の軽微な更新提案（誤読防止）

`3_reviewsForBasicDesign.md` の“齟齬・補足”に「DesignOverview は araneaDevices」記載が残っています。
これは第2回で解消済みなので、**履歴として残すなら「当時は〜、現在は統一済み」**にしておくとレビュー資料として強くなります。

---

# 4) Issue #111 に貼れる再レビューコメント（コピペ用）

* ✅ #111 記載の P0-1〜P0-4 / P1-2 は反映確認（fid=0000、Summary JSON有効化、TypeDomain=araneaDevice、JWT廃止→lacisOath、韓国語混入解消）   
* 🔁 追加で直すと安全（ドキュメント間整合）

  1. `03_authentication.md` の is22パラメータ表が旧表記（TypeDomain=araneaDevices / Type=ar-is22Camserver）で残存 → DesignOverviewと同一値へ統一推奨 
  2. `05_paraclate_integration.md` の Summary例が旧フォーマット（fendDetectAt / 配列形式）→ DesignOverviewの新フォーマットへ合わせる 
  3. `Paraclate_DesignOverview.md` の lacisOath サンプルが `json` なのに構文破損＋cic/fid混線 → JSONとして正規化推奨 

---

この3点を直せば、**「第2回レビュー対応：ドキュメント間整合性OK」**として #111 はクローズで問題ありません。
---

# review2
再レビューしました（Issue #110 / commit **310aac7**）。

---

## ✅ 今回の修正はOK（狙い通り）

### 1) is21 の fid 不一致は解消

`Paraclate_DesignOverview.md` の is21 所属が **fid=0000** に修正され、かつ「fid=0000は全施設を表す特殊値」と明記されました。
これは要件定義側の「is21は system tenant + fid=0000 + permission71」の記述とも整合しています。

### 2) レビュー対応履歴ファイルの追加は有効

`3_reviewsForBasicDesign.md` は、レビュー観点・判定・残存リスク・次アクションが整理されており、実装フェーズに入る前提確認として良いです。

---

## ❗️P0（まだ残っていて、このままだと実装事故る／仕様が割れる）

### P0-1) DesignOverview の JSON例が “JSONとして壊れている”

`Paraclate_DesignOverview.md` の Summary送信例が **コロン無し**等で機械可読ではありません。実装時にコピペ事故が起きます。
→ **必ず「機械可読JSON」に修正**（キー/型/必須/上限も固定）してください。

### P0-2) TypeDomain 表記ゆれは “残置”しない

DesignOverview に `Typedomain=araneaDevices` が残っています。
一方、基本設計は **TypeDomainは araneaDevice で統一**と明記しています。
レビュー対応側では「DesignOverviewは概念名として残置（実害なし）」と書いていますが、 **DesignOverviewは実装の参照元になるので実害が出ます（コピペ元）**。
→ **DesignOverview も araneaDevice に統一**してください。

### P0-3) “全ドキュメントでfid=0000統一” はまだ成立していない

`investigation_report/02_is21_status.md` は **FID: 9966**のままです。
→ 次のどちらかで **アンアンビギュアスに確定**してください（曖昧なままが一番危険）

* A案: 設計を fid=0000 に統一する（このファイルも直す）
* B案: 「所属(fid=9966)」と「権限(fidsに0000を含む)」を概念分離して、**どのフィールドがどっちなのか**を全ドキュメントに明記する

### P0-4) 認証方式が二重定義のまま（JWTヘッダ vs lacisOath）

`investigation_report/05_paraclate_integration.md` で `Authorization: Bearer {jwt_token}` を要求しつつ、bodyに `lacisOath` もあります。
→ **1方式に統一**してください（JWTを採用するなら、発行/更新/失効/誰が持つかまで仕様化しないと実装が止まります）。

---

## 次コミットで直すべき “最短チェックリスト”

1. `Paraclate_DesignOverview.md`

   * Summary送信例を **機械可読JSON**に直す（壊れている例を削除 or 修正）
   * TypeDomain を **araneaDevice**へ統一 
2. `investigation_report/02_is21_status.md`

   * FID:9966 をどう扱うか（A案orB案）に合わせて整合させる 
3. `investigation_report/05_paraclate_integration.md`

   * 認証方式を1つに固定（JWTかlacisOathか） 

---

## Issue #110 に貼り付け用コメント（コピペ用）

（このチャットから直接Issueへコメント投稿する操作が今の環境だと失敗するため、貼り付け文を置きます）

```md
統括アドバイザーとして commit 310aac7 を再レビューしました。

✅ OK
- DesignOverview の is21 fid を fid=0000 に修正し、特殊値の説明も追加されている（要件定義とも整合）。
- 3_reviewsForBasicDesign.md でレビュー観点/残存リスク/次アクションが整理されていて良い。

❗️P0（このままだと実装事故る）
1) DesignOverview の Summary送信例が JSON として壊れている（機械可読JSONに修正必須）。
2) TypeDomain 表記ゆれ：DesignOverview に araneaDevices が残存。実装参照元なので araneaDevice に統一必須。
3) fid統一の宣言が未達：investigation_report/02_is21_status.md が FID:9966 のまま。A)0000統一で更新 もしくは B)所属(fid)と権限(fids=0000含む)の概念分離を全ドキュメントで明記。
4) 認証方式が二重定義：05_paraclate_integration.md が JWTヘッダ + lacisOath 併存。1方式に統一。

→ 上記P0を潰し切ったら「レビュー対応完了 ✅」として実装フェーズに進めます。
```

---

# review3
## 再レビュー結果（Issue #111 / commit 515c509）

結論：**Issue #111 に列挙された P0（4件）＋P1（1件）は、いずれも修正反映を確認できました。** 
ただし **「ドキュメント間の整合性」観点で残っている差分が3点**あり、ここは次の事故（実装時の手戻り）に直結しやすいので、**追加で直してから #111 をクローズ**が安全です。

---

# 1) #111 の対応完了確認（P0/P1）

### P0-1: is21 の fid 統一（9966→0000）

`02_is21_status.md` の権限設定が **FID:0000** に修正され、注釈も追加されています。

### P0-2: DesignOverview の Summary JSON を有効化

Summary ペイロードが **有効なJSON**（`"key":"value"`）に修正され、構造もオブジェクト化されています。

### P0-3: TypeDomain を araneaDevice（単数形）に統一

DesignOverview 内の表記が **TypeDomain=araneaDevice / Type=ar-is22CamServer / Prefix=3** に統一されています。

### P0-4: 連携設計の認証方式を lacisOath に統一（JWT廃止）

`05_paraclate_integration.md` で JWT/Bearer を廃止し、**リクエストボディの lacisOath で認証**に統一されていることを確認しました。

### P1-2: 韓国語混入の解消＋SSoT表整形

`Paraclate_FunctionArrangement(AIchat).md` の SSoT表が整形され、「現在」に修正済み、LacisFiles 行も追加されています。 

---

# 2) 追加指摘（整合性レビューで残っている差分）

## (A) 【P0相当】03_authentication.md がまだ “旧表記” のまま（登録実装を誤誘導）

`03_authentication.md` の is22用パラメータ表が **TypeDomain=araneaDevices / Type=ar-is22Camserver / Prefix=3(araneaDevices共通)** のまま残っています。
一方で DesignOverview は **TypeDomain=araneaDevice / Type=ar-is22CamServer** に正規化済みです。

**なぜP0か**：AraneaRegister（Rust実装）で *typeDomain/type* を間違えると、mobes側の typeSettings / default permissions / UIフィルタで破綻しやすく、登録～権限付与がズレます（＝実装での致命傷）。

✅ **修正指示（アンアンビギュアス）**

* `03_authentication.md` の表（2.2）を **DesignOverview と同一値**に置換

  * `TypeDomain: araneaDevice`
  * `Type: ar-is22CamServer`
  * `Prefix: 3（araneaDevice共通）`

---

## (B) 【P1】05_paraclate_integration.md の Summary例が “旧フォーマット” のまま

DesignOverview は Summary JSON を新フォーマット（`cameraContext: object`, `cameraDetection: object[]`, `lastDetectAt`）に更新済みです。
しかし integration 側の例は `fendDetectAt` のまま、`cameraContext` が配列、`cameraDetection` がCSV文字列配列のままです。

✅ **修正指示**

* integration の例を **DesignOverview の送信形式と一致**させる

  * `fendDetectAt` → `lastDetectAt`
  * `cameraContext` → オブジェクト
  * `cameraDetection` → `{timestamp, cameraLacisId, detectionDetail}[]`

---

## (C) 【P1】DesignOverview の lacisOath 例が “JSONとして壊れている”

DesignOverview の `lacisOath` サンプルは ` ```json ` で囲われていますが、現状は **構文が壊れており**、さらに `cic` の値が `fid` と混線しています。

✅ **修正指示**

* ここは「仕様の核」なので、**JSONとして正しい形**に直す（最低限：`"lacisOath": { "lacisID": "...", "tid": "...", "cic": "...", "blessing": "..." }`）

---

# 3) 3_reviewsForBasicDesign.md の軽微な更新提案（誤読防止）

`3_reviewsForBasicDesign.md` の“齟齬・補足”に「DesignOverview は araneaDevices」記載が残っています。
これは第2回で解消済みなので、**履歴として残すなら「当時は〜、現在は統一済み」**にしておくとレビュー資料として強くなります。

---

# 4) Issue #111 に貼れる再レビューコメント（コピペ用）

* ✅ #111 記載の P0-1〜P0-4 / P1-2 は反映確認（fid=0000、Summary JSON有効化、TypeDomain=araneaDevice、JWT廃止→lacisOath、韓国語混入解消）   
* 🔁 追加で直すと安全（ドキュメント間整合）

  1. `03_authentication.md` の is22パラメータ表が旧表記（TypeDomain=araneaDevices / Type=ar-is22Camserver）で残存 → DesignOverviewと同一値へ統一推奨 
  2. `05_paraclate_integration.md` の Summary例が旧フォーマット（fendDetectAt / 配列形式）→ DesignOverviewの新フォーマットへ合わせる 
  3. `Paraclate_DesignOverview.md` の lacisOath サンプルが `json` なのに構文破損＋cic/fid混線 → JSONとして正規化推奨 

---

この3点を直せば、**「第2回レビュー対応：ドキュメント間整合性OK」**として #111 はクローズで問題ありません。
