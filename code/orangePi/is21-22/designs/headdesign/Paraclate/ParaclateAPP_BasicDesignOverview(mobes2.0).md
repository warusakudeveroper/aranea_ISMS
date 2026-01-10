# ParaclateAPP 要件書（基本設計作成前 / 概要＋注意点まとめ）

**作成日**: 2026-01-10
**対象**: ParaclateAPP 開発ライン（= mobes2.0 / Proclate 側のテナント管理層アプリ実装）
**前提**: SSoT / SOLID / MECE / アンアンビギュアス / 車輪の再発明禁止（The_golden_rules 準拠）

---

## 0. 目的（この要件書の役割）

本書は **ParaclateAPP の基本設計に入る前に、仕様の骨子・依存関係・境界条件・未決事項・注意点** を MECE に整理し、実装が「場当たり」にならないようにするための要件書である。
（＝“実装を始める前に、何を決め切ってから設計に入るか” を明文化する）

---

## 1. 正式名称・UI表記（必須）

* **正式名称**: Paraclate

  * セマンティック: 「そこにあって穏やかに見守り、対話的に報告する存在（監視/監査より “対話・報告” 寄り）」
* **ダッシュボード表記**: `Paraclate`
* **サブタイトル**: `mobes AI control Tower`

---

## 2. スコープ（ParaclateAPP が提供すること）

### 2.1 テナント管理層（Tenant Admin）の Paraclate 機能

1. **カメラ管理**

* カメラ（仮想 araneaDevice）一覧・状態・所属（tid/fid/rid）・基本情報（MAC/IP/ブランド/プリセット等）
* カメラコンテキスト（cameraContext）編集・保存・同期

2. **サマリー閲覧と設定**

* Summary（間隔ベース）
* GrandSummary（時間指定ベース）
* 両者を **別設定・別DB管理** とする（運用既定: Summary=間隔、GrandSummary=時刻指定）

3. **サーバー設定（is22 / is21連携）**

* is22（ParaclateEdge と連携する前段）の **設定の双方向同期**
* ステータスは Pub/Sub トピックによる（既存実装を確認し再発明禁止）

4. **AIアシスタント（チャットUI）**

* mobes2.0 の AI アシスタントタブから **Paraclate APP をバックエンドとして** 応答を返す
* 「今日何かあった？」「赤い服の人来た？」等、**過去ログ参照＋対話で範囲を詰める**UX
* **カメラ間の移動・不調傾向**など横断的洞察を返す（※根拠データ必須、捏造禁止）

5. **データパイプライン**

* 最終的に **検出ログ（event）とサマリー（summary）を BigQuery へ集約**し、解析強化する（イベントID/サマリーIDベース）

6. **LacisFiles（スナップショット保管）運用**

* 検出スナップショットを LacisFiles へ保存するが、**一定期間で削除しないと容量が破綻するため、保持・削除ポリシーが必須**
  （既存の「定期クリーンアップ」実装パターンを踏襲すること：例として DrawBoards は “30日保持のクリーンアップ” を実装している） 

---

## 3. 非スコープ（ParaclateAPP が “やらない” こと）

* カメラの映像処理アルゴリズム（検知・顔特徴・車両OCR等）の実装主体にならない
  → それらは **is21/22 ラインで実装**し、ParaclateAPP は **設定・結果参照・可視化・対話**に徹する
* “テナント越境” を ParaclateAPP が勝手に許可しない
  → 越境は **LacisOath 側の権能（blessing）実装が前提**（後述）

---

## 4. 用語・識別子（アンアンビギュアス定義）

### 4.1 LacisID（20桁固定）

* LacisID は **20桁固定**（編集不可） 
* araneaDevice系の “deviceWithMAC” 形式（参考）:
  `[Prefix=1桁][ProductType=3桁][MAC=12HEX][ProductCode=4桁]` 

### 4.2 tid / fid / rid / cic

* **tid**: テナント（契約主体）ID
* **fid**: 施設ID（4桁）
* **rid**: 部屋/リソースID（運用上の単位。cameraContext 等が紐づく可能性）
* **cic**: 認証コード（6桁など。用途により異なる）
* `fid='0000'` は **全施設アクセス**を意味し、System Admin 以外は付与不可 

### 4.3 認証オブジェクト命名の注意（重要）

“lacisOath” という語が **2種類の意味で混ざりやすい**ため、基本設計では必ず名称を分離する：

* **(A) Tenant Primary 認証（登録系）**: `lacisId + userId + cic`
* **(B) Device Auth（デバイス動作系）**: `tid + lacisId + cic`（report/ingest 等）

現状の araneaDeviceGate 実装では、`pass` は **廃止扱い**で検証しない（= `lacisId + userId + cic` で十分） 
→ Paraclate の「is22 → mobes2.0 認証」は、要件上 **(B) Device Auth 相当**が中心になるため、混同禁止。

---

## 5. 権限境界（tid越境禁止が大前提）

### 5.1 標準境界

* **テナント情報越境は許可されない**（tid境界を破る実装は禁止）
* 権限階層の目安（araneaSDK側の整理）
  `71+ System Authority / 61+ Tenant Primary / 41+ Tenant Management / 10 Staff` 

### 5.2 “blessing” による越境（要件としては存在、実装は未）

* 要件: permission91以上が特定lacisIDへ “越境権能” を付与し、is22が監視対象カメラへ越境アクセス可能にする
* 注意: **現時点で mobes2.0 / LacisOath にその実装がある前提で設計しない**
  → 基本設計前に「blessing の SSoT（どこに保存し、どのAPIが検証し、監査ログをどこへ出すか）」を確定する必要がある

---

## 6. データモデル（SSoTの指定）

### 6.1 userObject / userObject_detail を正とする

* デバイス（is22 / カメラ / etc）は **userObject / userObject_detail** で管理する（LacisOath正本）
* araneaDeviceGate の userObject 作成例でも、`tid / type_domain / fid[] / rid[] / permission / cic_code` 等が中核になっている 

### 6.2 TypeDomain / Type / ProductType / ProductCode の扱い（要注意）

* 既存の araneaDeviceGate の型定義は `typeDomain: 'araneaDevice'` を前提にしている 
  → 要件文に出てくる `Typedomain=araneaDevices` は **表記ゆれの可能性が高い**ため、基本設計前に **既存実装（typeDomainDefinitions / userTypeDefinitions / typeSettings）と整合**させること（新規TypeDomainを増やす場合はSSoT/移行計画必須）

---

## 7. UI/設定の配置（どこで何を設定するか）

### 7.1 施設コンテキスト（facilityContext）は既存 Settings にある

* `FacilitySettingsDialog` に **FacilityContext タブ**が存在する 
* Paraclate の対話応答は、この facilityContext を参照して「施設の前提」を補う（既存資産を活用する）

### 7.2 Paraclate 専用の “カメラ応答用コンテキスト” は新設

* facilityContext（既存）とは別に、ParaclateAPP が **cameraContext をテナント管理層で設定**する
* cameraContext は **カメラlacisIDをキー**にして保存し、is22 / is21 / Paraclate の参照に使う（SSoTを一箇所に寄せる）

### 7.3 「AIタブ」に Paraclate endpoint 設定を作る（最初にやる）

* mobes2.0 には AI Model Settings が存在し、SemanticTags（BQログ等）もここで設定している 
* ここに **Paraclateエンドポイントの設定フィールド**を追加し、初期値は `https://www.example_endpoint.com` とする
* ただし、AI設定は **tenant/facility override** があるため、Paraclate endpoint は

  * global default（プレースホルダー）
  * tenant override（本番URL）
    の二段構えを前提にする（後述）

---

## 8. AI設定（マルチステージLLM運用の要件）

### 8.1 aiApplications / overrides の仕組みを正として利用

AI Mode Settings は tenant/facility override を持つ（正本）：

* `/tenants/{tid}/aiModeOverrides/{appId}`
* `/facilities/{fid}/aiModeOverrides/{appId}` 
  → Paraclate も **aiApplications の appId として登録し、endpoint やプロンプト等を override 可能**にする。

### 8.2 マルチステージ動作（UX要件）

* 会話（チャット応答）: **Flash系（低レイテンシ優先）**
* 定期サマリー/グランドサマリー: **Pro系（整理・推論の質優先）**
* ただし「DBに情報が無い/未実装」でも、cameraContext と検出結果の範囲で **推論回答を返す**（=沈黙やエラーでUXを壊さない）

---

## 9. 定期サマリー（Summary / GrandSummary）の要件

### 9.1 設定

* Summary: 間隔ベース（例: 5分 / 15分 / 1時間）
* GrandSummary: 時刻指定ベース（例: 09:00 / 18:00）

### 9.2 DB要件

* Summary / GrandSummary ともに **設定と実行履歴（生成物）を別管理**する
* summary payload（案）は提示のJSONを初期案とし、**設計で固定キー・型・上限制約**まで確定する（アンアンビギュアス）

```json
{
  "summaryOverview": {
    "summaryID": "{summaryID}",
    "firstDetectAt": "{timeStamp}",
    "lastDetectAt": "{timeStamp}",
    "detectedEvents": "n"
  },
  "cameraContext": {
    "{camera_lacisID1}": {
      "cameraName": "{cameraName}",
      "cameraContext": "{cameraContext}",
      "fid": "{fid}",
      "rid": "{rid}",
      "preset": "{preset}"
    }
  },
  "cameraDetection": [
    {
      "timestamp": "{DetectionTimestamp}",
      "cameraLacisId": "{camera_lacisID}",
      "detectionDetail": "{detectionDetail}"
    }
  ]
}
```

---

## 10. BigQuery集約（イベントID/サマリーIDベース）

* 原則: “Proclateにイベントを送る” ではなく、**BQへ集約して後段で可視化/要約**する方向を優先
* ただし、Metatron が BQ に直接書き込むように（非ブロッキングで insert）実装パターンが既にあるため、再利用方針で検討する 

---

## 11. is21 / is22 との責務境界（最低限）

### 11.1 is21（ParaclateEdge）

* is21 は “レスポンスを返すエンジン” 側
* is22 → is21 に対して `tid/lacisID/cic` で activate
* **is22 に is21 の userObject 登録は不要**（= is21を参照するだけ）

### 11.2 is22

* is22 は **テナント配下の全fidにアクセスする権限**を持つ想定
* ただしその根拠（userObjectのfid配列設計/ルール/運用）を、mobes2.0の既存仕様と整合させる必要がある（“なんとなく0000扱い”は禁止）

---

## 12. 直近で “基本設計に入る前に” 確定が必要な論点（チェックリスト）

### 12.1 SSoT / データ配置

* cameraContext の正本はどこか（tenant配下？facility配下？userObject_detail配下？）
* Summary / GrandSummary の正本はどこか（履歴はどこか）

### 12.2 認証

* is22 → Paraclate の auth オブジェクトの正式スキーマ名（lacisOath ではなく DeviceAuth 等にするか）
* blessing の正本・検証・監査ログ（未実装なら “未実装” と宣言してフェーズ分割）

### 12.3 TypeDomain/Type/ProductType

* is22: ProductType=022, ProductCode=0000 を **既存 typeSettings と整合**させる
* camera(is801): ProductType=801 の登録・productCode運用（ブランド割当ルール）
* 既存 `typeDomain='araneaDevice'` で行くか、新TypeDomainを増やすか（増やすなら移行計画必須）

### 12.4 保存と削除（容量破綻対策）

* LacisFiles の保持日数 / 削除バッチ（Cloud Scheduler等）
* BQ 側の retention（パーティション/クラスタリング/削除方針）

### 12.5 UX（対話設計）

* 「過去ログどこまで遡る？」の対話プロトコル（確認質問のテンプレ）
* 根拠提示の粒度（日時・カメラID・スナップショットURLなど）

---

## 13. 受け入れ条件（最低限のDefinition of Done）

* tid/fid境界が破られない（越境は blessing がある場合のみ、かつ監査ログが残る）
* Summary/GrandSummary が設定通りに生成され、履歴が参照できる
* AIアシスタントで「今日何かあった？」→ BQ/Firestore参照に基づく回答が返る（未データ時はその旨を明示）
* スナップショット保存が動作し、保持期間を過ぎたデータが削除される（容量破綻しない）
* 実UI（Chrome）でのテスト証跡が残る（スクショ＋手順＋結果）

---

## 14. 統括アドバイザー所見（現時点での“注意喚起”）

* **lacisOath の意味が二重化している**ため、設計開始前に “登録系” と “デバイス動作系” の呼称を分離しないと、仕様事故が起きる（特に blessing を混ぜると破綻しやすい）。
* 既存実装上、登録系の `pass` は廃止扱いで検証しない  一方、古いドキュメントやサンプルに残存しているため、**Paraclate側で pass 必須の設計に戻すのは禁止**（互換崩壊を誘発する）。

---

必要なら、この要件書をそのまま **mobes2.0 側のドキュメント（Paraclate/requirements.md）形式**に整えて、「基本設計の章立て（データモデル/画面/IF/API/テスト）」へ落とし込む“設計インデックス”も続けて出します。
