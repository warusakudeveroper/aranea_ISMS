

# サマリー関連の精度向上

1)サマリーは検出結果(検出あった、なかった)のみを数えて通知しているがこれはis22から検出があった、無かったのみを受けていて実際のis21からの検出した詳細なパラメータをサマリーの対象としていない為か？
- 現在のサマリー内容
```
【定時サマリー】1/13 23:12〜1/14 00:12 検出: 2件（1台） 最大重要度: 4 1時間の検出サマリー: 合計2件の検出（1台のカメラ） - cam-94b491...: 2件 (最大severity: 4)
```
- 問題点
	- severityなどの用語を使用すべきではない
	- カメラ名ではなくカメラIDが使用されている
	- 簡素化されるべきではない部分が簡素化されている。
- 期待される表示内容
```
#【直近の状況】
## 【概要】
SummaryID:XXXXXXXXXXXXXXXXXXXX
1/13 23:12〜1/14 00:12
カメラの状態:15/17の正常性を確認しました
- **0150:HALE京都丹波口**　**2台がロスト**
  - Tam-Parking-N(192.168.125.13)
    最終疎通:MM/DD XX:XX:XX
    - Tam-Smoking(192.168.125.12)
      最終疎通:MM/DD XX:XX:XX
- **0151:HALE京都東寺**　全カメラ正常
- **0100:SORAPIA**　全カメラ正常
  
##【検出内容の要約】
2台のカメラが不通の状態を前回サマリーから継続してます。
Toj-Entranceのカメラで2件の検出があり、代表的な検出は以下です。//これは後述の検出内容を翻訳。
- 大人の女性が2人映ってるようです（0.51）
- 2人とも体格はがっしりめで髪は覆われ気味/しゃがみ姿勢/帽子っぽい＋メガネ/コート風の服にバッグ/リュックや手持ちあり、制服っぽさもあり。
- 1人は背低めで青の上着＋白ズボン、もう1人は身長普通で黒の上着＋黒ズボン
- さらにマスクっぽい判定。
- マスク＋しゃがみ要因で警戒度は高め。画面はシーン継続変化。
この人物特徴は1時間前にも検出しており、HALE京都東寺のデフォルトチェックインは15:00in-10:00outなのでスタッフもしくは滞在中ゲストであると考えられ、
直近数日の人物特徴レジストリに類似該当が130件あり(特徴ID:XXXXXXXXXX-XXXXに登録)
この人物はToj-1F-EVT、Toj-2F-EVTでも検出されており特定の部屋への動線以外で館内を移動していることからスタッフもしくは長期滞在者と判断
入口のQRコードリーダーなどでの滞留もないため
特にサポートの問題、侵入者による異常はないと判断して良いと考えられます。


##【総評】
23：12から0:12の状態報告としては2台のカメラの不通が継続してますが問題なしと判定して良いと思います。
以上が報告となります。
```

実際の検出データ内容サンプル(これはIS22にある)
```
{
  "analyzed": true,
  "bboxes": [
    {
      "conf": 0.5141051411628723,
      "details": {
        "body_build": "heavy",
        "body_size": "small",
        "bottom_color": {
          "color": "white",
          "confidence": 0.8985483870967742
        },
        "hair_type": {
          "confidence": 0.7,
          "type": "covered"
        },
        "height_category": "short",
        "index": 0,
        "meta": {
          "age_confidence": 0.729616105556488,
          "age_group": "adult",
          "gender": "female",
          "lower_type": "trousers",
          "upper_sleeve": "long"
        },
        "posture": {
          "confidence": 0.6,
          "type": "crouching"
        },
        "top_color": {
          "color": "blue",
          "confidence": 0.46914141414141414
        },
        "uniform_like": true
      },
      "label": "person",
      "par": {
        "meta": {
          "age_confidence": 0.729616105556488,
          "age_group": "adult",
          "gender": "female",
          "lower_type": "trousers",
          "upper_sleeve": "long"
        },
        "tags": [
          "appearance.hat_like",
          "appearance.glasses",
          "carry.backpack",
          "carry.bag",
          "carry.holding",
          "outfit.coat_like"
        ]
      },
      "x1": 0.4259412884712219,
      "x2": 0.5128127932548523,
      "y1": 0.17096659541130066,
      "y2": 0.3625762462615967
    },
    {
      "conf": 0.5136650204658508,
      "details": {
        "body_build": "heavy",
        "body_size": "large",
        "bottom_color": {
          "color": "black",
          "confidence": 0.95
        },
        "hair_type": {
          "confidence": 0.7,
          "type": "covered"
        },
        "height_category": "average",
        "index": 1,
        "mask_like": true,
        "meta": {
          "age_confidence": 0.7306743860244751,
          "age_group": "adult",
          "gender": "female",
          "lower_type": "trousers",
          "upper_sleeve": "long"
        },
        "posture": {
          "confidence": 0.6,
          "type": "crouching"
        },
        "top_color": {
          "color": "black",
          "confidence": 0.443003344340022
        },
        "uniform_like": true
      },
      "label": "person",
      "par": {
        "meta": {
          "age_confidence": 0.7306743860244751,
          "age_group": "adult",
          "gender": "female",
          "lower_type": "trousers",
          "upper_sleeve": "long"
        },
        "tags": [
          "appearance.hat_like",
          "appearance.glasses",
          "carry.backpack",
          "carry.bag",
          "carry.holding",
          "outfit.coat_like"
        ]
      },
      "x1": 0.3196994364261627,
      "x2": 0.6506027579307556,
      "y1": 0.04182300716638565,
      "y2": 0.5220093131065369
    }
  ],
  "camera_id": "cam-69679d22-b81e-476c-a993-5a62f5a0c030",
  "captured_at": "2026-01-13T10:32:47.027908427+00:00",
  "confidence": 0.5141051411628723,
  "count_hint": 2,
  "detected": true,
  "frame_diff": {
    "camera_status": "scene_change",
    "enabled": true,
    "loitering": null,
    "movement_vectors": null,
    "person_changes": {
      "appeared": 1,
      "disappeared": 5,
      "moved": 0,
      "stationary": 1
    },
    "scene_change": null
  },
  "is22_timings": {
    "is21_inference_ms": 151,
    "is21_par_ms": 18,
    "is21_roundtrip_ms": 566,
    "is21_yolo_ms": 32,
    "network_overhead_ms": 415,
    "save_ms": 0,
    "snapshot_ms": 2356,
    "snapshot_source": "ffmpeg",
    "total_ms": 2923
  },
  "output_schema": null,
  "performance": {
    "inference_ms": 151,
    "par_ms": 18,
    "yolo_ms": 32
  },
  "person_details": [
    {
      "body_build": "heavy",
      "body_size": "small",
      "bottom_color": {
        "color": "white",
        "confidence": 0.8985483646392822
      },
      "height_category": "short",
      "index": 0,
      "meta": {
        "age_confidence": 0.729616105556488,
        "age_group": "adult",
        "gender": "female",
        "lower_type": "trousers",
        "upper_sleeve": "long"
      },
      "posture": {
        "confidence": 0.6000000238418579,
        "type": "crouching"
      },
      "top_color": {
        "color": "blue",
        "confidence": 0.46914142370224
      },
      "uniform_like": true
    },
    {
      "body_build": "heavy",
      "body_size": "large",
      "bottom_color": {
        "color": "black",
        "confidence": 0.949999988079071
      },
      "height_category": "average",
      "index": 1,
      "meta": {
        "age_confidence": 0.7306743860244751,
        "age_group": "adult",
        "gender": "female",
        "lower_type": "trousers",
        "upper_sleeve": "long"
      },
      "posture": {
        "confidence": 0.6000000238418579,
        "type": "crouching"
      },
      "top_color": {
        "color": "black",
        "confidence": 0.4430033564567566
      },
      "uniform_like": true
    }
  ],
  "preset_applied": null,
  "preset_id": null,
  "preset_version": null,
  "primary_event": "human",
  "schema_version": "2025-12-29.1",
  "severity": 3,
  "suspicious": {
    "factors": [
      "appearance.mask_like (+40)",
      "crouching_posture (+10)"
    ],
    "level": "high",
    "score": 50
  },
  "tags": [
    "hair.covered",
    "top_color.blue",
    "carry.backpack",
    "outfit.shorts",
    "outfit.dress",
    "appearance.uniform_like",
    "outfit.pants",
    "height.short",
    "body.build.heavy",
    "appearance.mask_like",
    "bottom_color.black",
    "carry.holding",
    "body.size.small",
    "appearance.glasses",
    "outfit.coat_like",
    "bottom_color.white",
    "outfit.boots",
    "age.adult",
    "count.multiple",
    "height.average",
    "body.size.large",
    "posture.crouching",
    "appearance.hat_like",
    "carry.bag",
    "gender.female",
    "top_color.black"
  ],
  "unknown_flag": false
}
```


グランドサマリについて
- グランドサマリは個々の検出結果ではなくサマリを要約するサマリです。
- これは前回グランドサマリから最新のグランドサマリの間に起こった内容を報告する意図があります。

グランドサマリで期待される内容
```
# 【定時報告　23:00】
## 【概要】
### 期間:1/13 15:00〜1/14 00:00
15:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
16:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
17:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
18:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
19:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
20:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
21:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
22:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
23:00 SummaryID:XXXXXXXXXXXXXXXXXXXX
--9時間、9件のサマリあり--

## 【主要なトピック】
### HALE京都丹波口のエントランスでの85件の検出(Tam-Entrance)
本日は6室のチェックイン、2室の連泊があり推定16名が宿泊します(宿泊予約情報より)これは人物特徴レジストリからの判定でも85件の検出から18人のエントリがあり宿泊ゲストとスタッフ、検出誤差の範疇に収まり、健全な状況といえます。

### HALE京都東寺のエントランスでの97件の検出(Toj-Entrance)
本日は3室のチェックイン、4室の連泊があり推定10名が宿泊します(宿泊予約情報より)これは人物特徴レジストリからの判定でも97件の検出から9人のエントリがあり宿泊ゲストとスタッフ、検出誤差の範疇に収まり、健全な状況といえます。
ただし17:00のサマリーにある16:45から15分間の滞留があり、何らかの問題があった可能性があります。この人物はその後Toj-Front,Toj-1F-EVT,Toj-2F-Eastで類似特徴(本日新規登録の特徴ID:XXXXXXXXXX-XXXX)が検出されてますのでQRコードリーダーの使い方などで問題があったもののチェックインし部屋に入ったものと思われます。

### カメラの健全性の問題
2台のカメラ
**0150:HALE京都丹波口**　**2台がロスト**
Tam-Parking-N(192.168.125.13)
最終疎通:MM/DD XX:XX:XX
Tam-Smoking(192.168.125.12)
最終疎通:MM/DD XX:XX:XX
このカメラは過去10回の定時報告で継続的に報告されており故障もしくは通信環境の問題がある可能性があります。早めに技術担当者への問い合わせを推奨します。

##【報告まとめ】
15:00〜00:00の報告は以上になります。
全体的には健全な状態ではありますが、16:45からの`Toj-Entrance`の滞留についてその時間帯のスタッフに確認を行った方が良いかもしれません。
2台のカメラの不調に関しては何らかの確認を行うことを推奨します。

```


## 人物特徴レジストリ(新規実装依頼)
- これは人物特徴をParaclate側で保存するものです。
- 人物特徴データは検出で偏る可能性がありますので必ずしも100%特定の人物を示せないことを前提として多少曖昧にしておく必要があります。
- firstDetected=その特徴データが登録されたタイミング
- lastDetected=その特徴データが最後にマッチしたタイミング
- LLM側でlastDetectedが1週間程度(これはParaclate側で設定として調整可能にして保存すべき)を超える場合は参照としては他人？の可能性が高いと判断,ただし完全に近く一致する場合は同一人物？など判定ロジックの調整が必要。
- 逆にカメラ画角などによる判定の違いが発生するのでどの程度を同一人物判定するかの検討も必要、例えば同じ施設なので近しい時間帯の検出で特徴に共通点がある場合はマージするなど？

person feature registry
```
"personFeatureID":"XXXXXXXXXX-XXXX",
"firstDetected":"{timestamp}",
"lastDetectedAt":"{timestamp}",
"feature":{
///ここにfeaturedata///
},
"meta":{
///ここにメタデータ///
},
"EventID":[
"{timestamp}:{eventID}}",
"{timestamp}:{eventID}}",
"{timestamp}:{eventID}}",
"{timestamp}:{eventID}}",
"{timestamp}:{eventID}}",
"{timestamp}:{eventID}}",
]

```

# チャットの継続会話について
サンプル

```
こんにちは、施設管理ご担当者様。 わたくし、施設監視・分析AIアシスタントのParaclateです。 HALE京都丹波口のカメラについてですね。承知いたしました。 現在、HALE京都丹波口には**8台**のカメラが設置されており、すべて正常に稼働しております。 各カメラの設置場所は以下の通りです。 * **Tam-Entrance-E2E-Test**: エントランス上部（施設への唯一の入口） * **Tam-1F-Reception**: エントランスを入ったレセプションエリア * **Tam-1F-Front**: フロントカウンター（内側から撮影） * **Tam-1F-EVT**: 1階エレベーターホール（101, 104-106号室周辺） * **Tam-2F-EVT**: 2階エレベーターホール（201-206号室周辺） * **Tam StorageArea**: 1階の事務所横、アメニティ在庫などが置かれている通路 * **Tam-Parking-N**: 外部駐車場 * **Tam-Smoking**: 喫煙所 以上がHALE京都丹波口のカメラの数と配置状況です。 何かお気づきの点や、さらに詳しく分析が必要な箇所がございましたら、いつでもお声がけください。 _応答時間: 34462ms_

18:24

直近で誰かエントランスに来た？

18:25

はい、承知いたしました。施設監視・分析AIアシスタントのParaclateです。 直近のエントランスの状況についてですね。 現在、HALE京都丹波口とHALE京都東寺の2つの施設を監視しておりますが、どちらのエントランスの状況についてお調べいたしましょうか？ 参考情報として、施設全体では直近24時間に53件の人物検出が記録されております。 ご確認になりたい施設名をお知らせいただけましたら、該当するエントランスカメラの記録を詳しく確認し、最新の来訪状況についてご報告いたします。 _応答時間: 21848ms_

18:25

丹波口

18:26


こんにちは、施設監視・分析AIアシスタントのParaclateです。 HALE京都丹波口の状況についてお知らせしますね。 まず、丹波口施設に関連する8台のカメラはすべて正常に稼働しており、映像は問題なく受信できておりますのでご安心ください。 直近24時間の施設全体（東寺を含む）の活動サマリーは以下の通りです。 * **総検出数**: 239件 * **人物検出**: 52件 * **車両検出**: 0件 これらの検出は、主にゲストやスタッフの方々の通常の出入りによるものと考えられます。 特に、チェックインでお困りの方の兆候となるエントランスでの長時間の滞留や、その他注意が必要なイベントは検知されておりません。穏やかな状況が続いているようです。 引き続き、各エリアの状況を注意深く見守ってまいります。 何かご不明な点や、さらに詳しく確認したい項目がございましたら、お気軽にお申し付けください。 _応答時間: 27796ms_

18:26
```

問題点
- 直近で誰かエントランスに来た？→どっちの施設かとAIから返信(ここまでは良い)→施設名答える(ここまでは良い)→施設のカメラの状態を回答
  となっており、完全に以前のメッセージコンテキストを参照していない


# ここからの実施方針
- まずサマリー関連の精度向上に関しては努力目標の話をしているのではなく"現在の状態だと使い物にならない状態"であることを把握する
- データフローとしてis22から必要な検出データをすべてParaclateに送信しているかを確認する。
- データフローとしてis22からの検出データをすべてParaclateが保存しているかを確認する。
-  データフローとしてis22からの検出データをすべてParaclateがLLMで参照できるようにしているかを確認する。
- そもそもParaclateがLLMで継続会話のためにメッセージ生成時に過去会話を参照しているかを確認する。
- ParaclateがLLMで上記のように回答を行うためにツール整備、ロジック整備が必要であればその必要項目を確認する。
- 確認結果から必要な設計項目を完全に作成し設計実施、MECEかつアンアンビギュアスであること、特に要件の完全達成に関しては一切の積み残しとならないように注意する。
- 以降はルールに従って実行すること
- mobes2.0,is22開発の情報のやり取りについては必ずmobes2.0側のdoc/APPS/Paraclate/LLMのリモートリポジトリでのやり取りとし、コミットプッシュして相互が誤った内容とならないように進める。コミットプッシュ時はパスとコミットIDを通知する。