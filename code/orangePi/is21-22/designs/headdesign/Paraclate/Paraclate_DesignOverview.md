# Paraclate実装について
#  正式名称
## 正式名称について
現時点をもって開発名をMobesAIcamControlTowerから
"Paraclate"に変更する。
Paraclateはギリシア語由来のそこで見守る精霊を表し、このカメラシステムの存在意義コンテキストとして常にそこにあって穏やかに見守り報告する、福音と啓示をもたらすような存在として若干監視・監査よりゆるく、対話的に語りかけてくるような存在であることがこの名称のセマンティックコンテキストとして認識すること。
## 表示・表記について
ダッシュボードページの表記としてはParaclate
サブタイトルとして mobes AI control Towerとする

## 以降の作業について
- 現時点でParaclateエンドポイントをhttps://www.example_endpoint.comのプレースホルダーとしてAIタブに設定フィールドを作成
- Paraclate定時報告は間隔ベースと設定時間ベースを設定をSummary、GrandSummary
  個別に設定、基本的な運用としてはSummaryは間隔、GrandSummaryは時間指定
  Summary、GrandSummaryともにDB整備、
   Summaryの送信形式案は以下の通り
   ```json
   
  {
"summaryOverview":{
	"summaryID","{summaryID}",
	"firstDetectAt","{timeStamp}",
	"fendDetectAt","{timeStamp}",
	"detectedEvents":"n"
	},
"cameraContext":{
	"{camera_lacisID1}":["{cameraName}","{cameraContext}","{fid}","{rid}","preset"],
	"{camera_lacisID2}":["{cameraName}","{cameraContext}","{fid}","{rid}","preset"],
	"{camera_lacisID3}":["{cameraName}","{cameraContext}","{fid}","{rid}","preset"]
	},
"cameraDetection":[
	"{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
	"{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
	"{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
	"{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
	"{DetectionTimestamp},{camera_lacisID},{detectionDetail}",
	]
}
```
送信時には
```json
{
lacisOath:
	"lacisID":"is22Device_lacisID",
	"tid":"is22Device_tid",
	"cic":"is22Device_fid},
	"blessing":"///これはlacisOath側で実装予定の機能、permission91以上のユーザーの権限において特定のlacisIDに対してtid権限越境の権能を付与する、このblessによってis22はテナント越境したカメラへのアクセス権を得ることができる。このフィールドはis22がテナント越境での監視を行う場合にのみ使用する"
}
```
の形式でmobes2.0の認証を行う。
- tid,fid,lacisID,cicの取り扱いなどについてはaraneaSDKのナレッジおよびMetatronコマンドなどを用いて確認必要
- 現時点ではis22にaranearegistor系の機能実装を行なっていないのでここについても実装タスクを作成する。code/ESP32/global/src/AraneaRegister.cppの実装を参考にすること。現在位置としてはまだスキーマ整備も行われていない。
- Typedomain=araneaDevices,Type=ar-is22Camsetver,Prefix3(araneaDecvices共通)Producttype=022,ProductCode=0000(末尾の追い番ないので4進ルール的仕様として0000)
- これまで開発環境としてlacisOath認証による権限境界を意識しない仕様として進めてきたが以降はtid権限境界、fid=施設としての管理を必要とする。
- 詳しくはhttps://github.com/warusakudeveroper/mobes2.0のlacisOath仕様関連の把握が必要である。
- 簡易的に解説するとtid=テナントIDでありmobes2.0の契約主体を示す、テナントの権限はテナントの中のみであり、テナントの情報越境は許可されない。fidは施設IDでありテナントは必ず1つ以上のfidを有する。fidのサブネットはmobes側に今後実装するParaclateAPPとis22Server側の双方で同期する。
- 現在fid9000の市山水産がis22に登録されているがこれは違うtidに所属しているため以降は以下の所属として扱う。
  `mijeo.inc tid T2025120621041161827
  テナントプライマリ:soejim@mijeos.com
  テナントプライマリlacisID:18217487937895888001
  テナントプライマリCIC:204965`
- is22およびis21の傘下のカメラ類はlacisOathの
- is22はそのtidが持つ全fidのアクセス権限を持つ
- is22のuserObjectDetailスキーマでは傘下のカメラのlacisIDリスト、Paraclateでの設定項目を値として持つ。
- 最終的に検出ログの全てをBQにポストする(サマリー系を強化する方向)、この場合にイベントID,サマリーIDベースとなりProclateにイベントを送信する必要はなくなるのでこの部分については検証が必要。処理としてはそちらの方が良い。
- カメラ設定でのカメラ側のパラメータについては各カメラが仮想のaraneaDeviceとしてis801 paraclateCameraとして振る舞うためlacisIDを3801{MAC}{ProductCodeはカメラブランドで割り振る}として独立したuserObject,userOBjectDetailで管理する。is801 paraclateCameraもTID,FIDに所属する。
- ipアドレスとMACについてはCelestialGlobeからも参照される。
- サーバーの設定関連については設定の双方向同期、ステータスについてはPUB/SUBトピックで行う。車輪の再発明にならないように既存実装を確認する
- AIアシスタントタブからの質問に関してもParaclate APPからのレスポンスでチャットボット機能を行う
- AIアシスタントの機能としてはサマリーを報告してくれるの他に
  「今日は何かあった？」
  →「本日は機能と比べて来訪者が多い日でした。カメラ上の検知としては宿泊ゲスト、スタッフが主で・・・(説明)」
  など**検出特徴の人物のカメラ間での移動などを横断的に把握する**
  **カメラ不調などの傾向も把握する**
  「赤い服きた女の人来なかった？」
  →「直近だとMM/DDのHH:mm:ssの記録でXXXXXXとXXXXXで検出されてます、もっと以前の記録まで遡って確認しますか？」
  など**過去の記録を参照する、過去の記録範囲を対話的にユーザーと会話する**
## is21の取り扱いについて
- is21も正式名称を"ParaclateEdge"が正式名称となる。
- 現在のis21も同様にlacisIDを持つがT9999999999999999999のシステムテナント、fid=0000のシステム専用論理施設(実施設ではなく管理上の論理施設。fid=0000は全施設を表す特殊値)に所属するUserObjectである。このデバイスはPermission71の権限を持つ特殊デバイスでありPermission71はクロステナント権限と全施設アクセス権限を持つため複数のtidからの処理に応じることができる。
- is21に関しても管理ページ(is20s相当)を持つ必要があるがそれらは仮実装であり現時点以降を持って設定管理を実装する必要がある。
- is22との連携についてはis22側からis21のサーバーアドレスのエンドポイントにアクセスしてis22のtid,lacisID,cicでアプティベートする。is21サーバー側ではアクティベート済みis22を管理可能とする。
- is21はレスポンスを返すだけなのでis22にis21のlacisID,fid,cic,tidを登録する必要はない。
- - サーバーの設定関連については設定の双方向同期、ステータスについてはPUB/SUBトピックで行う。車輪の再発明にならないように既存実装を確認する

# 今から行うこと
- 上記に基づいて大枠として行うことは以下である
## 統括アドバイザー
- https://github.com/warusakudeveroper/mobes2.0
  および
  https://github.com/warusakudeveroper/aranea_ISMS
  の内容を確認検証し相互の齟齬のないMECEな構成とメソッド、相互の機能実装の洗い出しを行いis21/22開発ラインとProclate(mobes2.0ダッシュボード)開発ラインの相互のタスクを洗い出す。
## is21/22開発ライン(あなた)
- 今回の最終実装フェーズの内容を整理、関連するドキュメントおよびGithubリポジトリの確認、現在の実装必要項目をcode/orangePi/is21-22/designs/headdesign/Paraclate/nvestigation_reportに項目別に整理してまとめインデックスを作成、統括アドバイザーの指示を待つ
- `The_golden_rules.md`の内容を意識する
## ParaclateAPP開発ライン
- テナント管理層に位置するProclateAPPの実装計画は統括アドバイザーの検証と指示に従う。
- Paraclateではカメラの管理、カメラサマリーの要約閲覧、カメラの設定、サーバーの設定管理を行う機能でありイベントの画像の保管保存=LacisFilesで行うがある程度の頻度で捨てないとユーゼージがパンクする。
- Paraclateの表示インターフェース側の整備はis22,21のaiチャット系の機能が完全に拡充してから詳細な再計画を要しますのでバックエンドを重点的に整備します
- https://mobesorder.web.app/system/ai-model
  のアプリ設定にAIモデル関連の基本的な仕様は保持、テナントごとのコンテキストは
  https://mobesorder.web.app/settings
  の情報、facilitycontext
  カメラ関連の応答用コンテキストはテナント管理層に新設するParaclateAPPでで設定
- 現在は曖昧な実装となっているが車両No.OCRと顔認識特徴データをis21に実装しますのでそのコンテキストを含めOperation Report to Tasksの実装内容(多段参照LLM)を確認。ユーザーからの質問→facilityContextとCameraContextから必要な情報参照先を確定、現在未実装のhttps://mobesorder.web.app/rooms(部屋の使用状況)、https://mobesorder.web.app/reservations(予約情報)、https://mobesorder.web.app/celestial-globeを参照(カメラが不通の時などはネットワーク情報参照)、https://mobesorder.web.app/horarium(シフト情報)を参照など
  Metatronの全コレクション、BQ参照系機能と連携
  最終的な結果内容を会話ではFlash系(応答速度が重要)サマリーではPro系(カメラの検知パラメータの整理が必要なので思考系)といったマルチステージでのLLMの動作が必要です。また未実装やデータベースに情報がない状態からの情報が得られなくても検出結果とカメラコンテキストからの推論回答が必要です。
- またスナップショットデータのLacisFilesへの保存保管、検知データのBQへの保存などの整備が必要です。
- APPSHELLサイドバーのテナント管理層にParaclate  APPの実装が必要です


# The_golden_rules.mdの順守
実装と調査に関しては以下のルールの遵守が必要です。
```
### 絶対遵守の大原則(必ず作業前と作業後にこことの照合を行う)

  

1. SSoTの大原則を遵守し同一内容のデータソースを乱立しない

  

2. Solidの原則を意識し特に単一責任、開放閉鎖、置換原則、依存逆転に注意

  

3. MECEを意識、このコードはMECEであるか、この報告はMECEであるかを報告の中に加える

  

4. アンアンビギュアスな報告回答を義務化しアンアンビギュアスに到達できていない場合はアンアンビギュアスであると宣言できるまで明確な検証を行う

  

5. 根拠なき自己解釈で情報に優劣を持たせる差別コード、レイシストコードは絶対の禁止事項です。情報は如何なる場合においても等価であり、優劣をつけて自己判断で不要と判断した情報を握り潰すような仕様としてはならない。全てのユーザーは情報を確認する権利を有する。ただし誤操作防止などの理由がある場合はこの限りではない。

  

6. "ファイルがあるからヨシ","見てないけど他の依存が大丈夫だからヨシ"、"とりあえず動かすのにハードコードで値取得したフリしてるけど動いてるからヨシ"といった"現場猫案件"は禁止

  

7. 優先順位は依存関係と実装順によってのみ定義される。棚上げにして将来実装などというケースはありません。

  

8. 実施手順と大原則に違反した場合は直ちに作業を止め再度現在位置を確認の上でリカバーしてから再度実装に戻る

  

9. チェック、テストのない完了報告は報告と見做されない

  

10. 確証バイアスやサンクコストに絡め取られて観測された現象を否定してはならない

  

11. 使えない、仕様上の問題あるけど仕様だからOKといった問題解決を見ない矮小化は禁止する

  

12. 依存関係と既存の実装を尊重し、確認し、車輪の再発明行為は絶対に行わない

  

13. タスクの開始時の契約として必ずこの実施手順と大原則を全文省略なく確認し、タスク終了時に宣言が履行されているか報告することをタスクにおける契約とする

  

14. 要件を変更しゴールを動かすことは禁止。

  

15. 設計ドキュメントのない実装、タスクリスト、テストリストの無い受け入れ条件が曖昧な実装は禁止です。

  

16. ルール無視はルールを無視した時点からやり直すこと。

  

17. 報告は必ず日本語で行うこと

  

18. 何かのトラブルがあった際は外的要因や環境要因を疑う前に必ずコードを疑うこと

  

19. araneaDeviceGate,認証登録系の登録、実証とテストはaranea_SDKを使用する。

  

  

### あらゆるアクションに共通する基本の手順(復唱不要)

  

1. 指示があった場合は必ず現象の確認もしくは現在の実装の確認を行う。

  

2. よほど簡易なタスクでない限りは必ず該当の設計、タスクに関する概要ドキュメントを作成する。ただし軽微なタスクに冗長な手法を採用する必要はない

  

3. その後必ず概要ドキュメントと実体、実装の整合性が取れているか確認する。

  

4. 概要ドキュメントと実体、実装の生合成が確認できたら項目ごとの詳細かつアンアンビギュアス、MECEな詳細設計を作成する。

  

5. 設計インデックスとタスクリスト、テスト計画を作成する。テストは必ずフロントエンド、バックエンド、chromeアクセスでの実UI/UXのテストとし、テストの簡素化は原則として認められない。

  

6. これら全てが揃ったらGithubにIssue登録する。

  

7. その後実装前の段階を必ずコミットプッシュする。

  

8. 実装を開始する。必ず全フェースにおいて`大原則の宣言`の複勝を行う

  

9. テストまで実行し報告する
```