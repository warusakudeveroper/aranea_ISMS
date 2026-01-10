# review1


再レビュー（commit **310aac7** / Issue **#110**）です。
結論：**fid=0000 修正そのものは妥当でOK**。ただし、Issue本文にある「確認済み整合性」には **まだ事実とズレている点が残っています**（ドキュメント上の齟齬／実装誘導ミスになり得る）。下記を直せば、レビュー完了にできます。

---

## ✅ 今回の修正（合格）

### is21 の fid を `fid=0000` に統一

`Paraclate_DesignOverview.md` で **is21 を system tenant + fid=0000（全施設を表す特殊値）**として明記できており、クロステナント権限（Permission71）と整合します。 
この変更自体は **良い修正**です。

### レビュー対応履歴ドキュメント追加

`3_reviewsForBasicDesign.md` は、観点・対応履歴・残存リスク・アクションがまとまっていて有用です。 

---

## ❗ ただし「整合性確認済み」宣言に残る問題（P0/P1）

### P0-1) 「fid=0000に全ドキュメント統一」は未達

`investigation_report/02_is21_status.md` の **設計上の権限設定**がまだ `FID: 9966` のままです。 
→ **どちらかに統一**してください。

* もしこのファイルが「現状（As-is）レポート」なら
  「現状: FID=9966、設計(To-be): FID=0000」みたいに**章を分けて明示**（アンアンビギュアス化）。
* もし「設計（To-be）」を名乗るなら **FID=0000 に直す**。

---

### P0-2) DesignOverview のサンプルJSONがまだ “JSONとして不正”

`Paraclate_DesignOverview.md` の Summary 送信例は、キーと値が `":"` ではなく `","` になっていて、そのまま実装できません。 
さらに認証部も `lacisOath:` の書き方や `cic` の値が壊れており、JSONではありません。 

→ **このままだと “コピペ実装事故” が起きる**ので、DesignOverview側も修正必須です（Golden rules #4/#15）。

**修正案（このまま貼れる Valid JSON）**：

```json
{
  "summaryOverview": {
    "summaryID": "{summaryID}",
    "firstDetectAt": "{timeStamp}",
    "fendDetectAt": "{timeStamp}",
    "detectedEvents": "n"
  },
  "cameraContext": {
    "{camera_lacisID1}": ["{cameraName}", "{cameraContext}", "{fid}", "{rid}", "preset"],
    "{camera_lacisID2}": ["{cameraName}", "{cameraContext}", "{fid}", "{rid}", "preset"],
    "{camera_lacisID3}": ["{cameraName}", "{cameraContext}", "{fid}", "{rid}", "preset"]
  },
  "cameraDetection": [
    "{DetectionTimestamp},{camera_lacisID},{detectionDetail}"
  ]
}
```

---

### P0-3) TypeDomainの“残置OK”はNG（設計ドキュメントは実装誘導するので）

`3_reviewsForBasicDesign.md` に「DesignOverview は `araneaDevices` だが概念名として残置（実害なし）」とありますが、これは危険です。 
理由：araneaSDK APIリファレンスで、`typeDomain: 'araneaDevice'` が**型として明確に固定**されています。 
実際に DesignOverview には `Typedomain=araneaDevices` の記述も残っています。 

→ **DesignOverview も araneaDevice に統一**してください（“概念名だからOK”は現場猫案件になりやすい）。

---

### P1-1) 認証方式がまだ混在（Bearer jwt_token）

`05_paraclate_integration.md` が `Authorization: Bearer {jwt_token}` を前提にしています。 
しかし、araneaSDK / mobes2.0 の既存実装では、デバイスからの送信は **DeviceAuth（tid + lacisId + cic）**が基本です。  

→ ここは方針を一本化してください：

* **登録（araneaDeviceGate）**＝ LacisOath（Tenant Primary） 
* **運用送信（ingest等）**＝ DeviceAuth（tid+lacisId+cic） 
  ※Bearerは「管理UI（人間）」の操作に限定するなら別レーンとして明記。

---

### P1-2) 日本語縛り違反（韓国語混入）＋SSoT表のブレ

`Paraclate_FunctionArrangement(AIchat).md` に韓国語 “아직” が混入しています。 
加えて同ファイルのSSoT表で “is22 MariaDB（camerasがSSoT）” と書かれていますが、is22の実装コメントでは **SSoT=ConfigStore** と明記されています。  

→ このファイルは **（1）日本語へ修正**、（2）SSoT表記を **ConfigStore正本（永続=MariaDB）**に統一、が必要です。

---

## 判定

* **fid=0000修正の妥当性**：✅ OK（今回の主旨は達成） 
* **ドキュメント間整合性が完全に取れたか**：❌ まだ（上のP0-1〜P1-2が残る）
* **残存リスクの棚卸し**：✅ 良い（3_reviewsで明示されているのは良い） 

---

## Issue #110 に貼るレビューコメント（コピペ用）

```text
再レビュー（commit 310aac7）

✅ fid修正（is21: fid=0000）は妥当でOK。DesignOverview側でも「fid=0000は全施設の特殊値」を明記できている。

ただし「整合性確認済み」にはまだ未達があるため、以下の修正が必要。

[P0]
1) “全ドキュメントでfid=0000統一” は未達
- investigation_report/02_is21_status.md が FID:9966 のまま。
→ As-isなら「現状=9966 / 設計=0000」を明記、To-beなら0000へ修正。

2) DesignOverview のサンプルJSONがJSONとして不正
- "summaryID","{summaryID}" 形式など、コピペ実装できない。
→ Valid JSONに修正（認証部も含めて）。

3) TypeDomain “araneaDevices” 残置はNG
- API_REFERENCEで typeDomain='araneaDevice' 固定。
→ DesignOverview含めて araneaDevice に統一（概念名だからOKは危険）。

[P1]
4) 認証方式が混在（Bearer jwt_token）
- device送信は DeviceAuth(tid+lacisId+cic) が既存正。
→ 05_paraclate_integration の認証を DeviceAuth へ寄せ、Bearerは管理UIレーンに限定するなら明記。

5) 日本語縛り違反・SSoT表記ブレ
- Paraclate_FunctionArrangement(AIchat).md に韓国語混入、SSoT表でMariaDB正本のように読める箇所あり。
→ 日本語化＋SSoT=ConfigStore（永続=MariaDB）に統一。

上記が直れば、レビュー完了で良いです。
```

この5点を次コミットで直せば、#110 は「レビュー完了」にできます。


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

## 再レビュー結果（Issue #110 / commit 310aac7）

**結論**: ✅ **fid不一致の修正は妥当で、#110 の指摘点はクローズ相当**です。
ただし、次フェーズで事故りやすい“残存リスク”がいくつかあるので、**#110の本文に書かれている「次回レビュー対象」＋追加2点**を、別Issueかタスクとして固定するのが安全です。

---

## 1) 差分確認（やったことが正しく反映されているか）

### is21 fid の不一致修正

* `Paraclate_DesignOverview.md` の is21 所属が `fid9966 → fid=0000` に置換され、**fid=0000 が「全施設を表す特殊値」**である注釈も追加されています。
* main ブランチ上の実ファイルも `fid=0000` 記載になっています。

### レビュー対応ドキュメント追加

* `3_reviewsForBasicDesign.md` が追加され、レビュー観点・判定・アクションアイテム・残存リスクが明文化されています。

---

## 2) 「fid=0000 へ統一」は妥当か？

妥当です。理由は2つあります。

1. **仕様・設計上の整合**
   is21 は system tenant / permission71 前提でクロステナント処理を担う、という方針が要件定義にも明記されています（`fid=0000` のまま応答）。

2. **9966 を system 専用にするのは衝突リスクがある**
   mobes2.0 側の facility 一覧に `fid 9966` が “実在施設” として載っており（黑猫的业务工具实验室）、今後も利用され得ます。
   なので「is21=9966」は将来の齟齬を誘発しやすく、**system は 0000 に寄せる**方が安全です。

> 1点だけ注意：DesignOverview の文言が「fid=0000 のシステム専用“論理施設”に所属」と読めます。
> 実装上は「所属施設」というより **“アクセススコープ（全施設）”**の意味合いが強いので、次のどちらかに寄せると誤読が減ります（推奨）
>
> * 「`lacis.fid` に `0000` を含む（全施設アクセス）」
> * 「system tenant / permission71 / facility scope = all」

---

## 3) 「3ドキュメント間の整合」は取れているか？

取れています（少なくとも #110 の論点である fid についてはOK）。

* DesignOverview: is21 が `fid=0000` と明記済み。
* RequirementsDefinition: FR-09 で is21 が `fid=0000 permission71` と明記済み。
* レビュー対応履歴にも、修正前後がテーブルで残っている（監査性◎）。

---

## 4) 残存リスク（#110記載の通り “次フェーズ対応” が妥当）

`3_reviewsForBasicDesign.md` に挙がっている残存リスクは、優先度付けも妥当です。特に **Paraclate ingest/chat のURI未確定が「結合試験ブロッカー」**なのはその通り。

### 追加で「次回レビュー対象」に入れてほしい2点（軽微だけど事故りやすい）

1. **TypeDomain 表記ゆれを DesignOverview 側でも潰す**
   DesignOverview に `Typedomain=araneaDevices` が残っています。
   3_reviews では「概念名として残置（実害なし）」扱いですが、ここはコピペされがちなので、**DesignOverview も `araneaDevice` に統一**するのが安全です。

2. **認証“用語”の分離（lacisOath と device cic の混線防止）**
   基本設計の冒頭で `lacisOath（lacisId/userId/cic）` を基本と定義している一方、Ingest API は「デバイスCIC + tid整合」になっています。 
   実装時の事故（誤って tenant primary を is22 に持たせる等）を防ぐため、ドキュメント上の呼称を

   * `tenantPrimaryAuth`（= lacisId/userId/cic）
   * `deviceAuth`（= lacisId/cic + tid整合、blessing optional）
     のように分けておくのが推奨です。

---

## Issue #110 に貼れる再レビューコメント（そのままコピペ可）

* ✅ `fid9966 → fid=0000` 修正は妥当（system tenant / permission71 / 全施設アクセスの特殊値として整合）
* ✅ DesignOverview / RequirementsDefinition 間の fid 不一致は解消確認
* ✅ `3_reviewsForBasicDesign.md` 追加でレビュー対応の監査性が上がった
* ⚠️ 次フェーズ持ち越し（本文記載の通り）

  * ingest/chat URI 未確定は結合試験ブロッカー
  * araneaDeviceGate の typeDefaultPermissions / fid渡し仕様変更は mobes 側依存
* 🔁 追加提案（軽微だが事故防止）

  * DesignOverview の `araneaDevices` を `araneaDevice` に統一推奨
  * 認証用語（tenantPrimaryAuth / deviceAuth）を分離して混線防止

---

このまま **#110 は「fid修正レビュー完了」扱いでOK**です。次は、`3_reviewsForBasicDesign.md` のアクションアイテム（特に **URI確定**）を “依存順” でIssue化していく段階に入れます。



---
