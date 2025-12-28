

# CamServer / CamImageEdge 基本設計書（Draft v0.1）

* 作成日: 2025-12-28
* 対象: RTSPカメラ30台の監視（スナップ更新/インデックス化/検索/推論/通知）
* 前提: 本設計は「ローカルWebサーバ」運用を基本とし、クラウド連携は Google Drive（画像保管）と必要時の外部LLM（画像理解）に限定する。

---

## 0. 用語・呼称

### ノード

* **ar-is21o / camimageEdge ai（is21）**

  * 機材: Orange Pi 5 Plus
  * OS: Armbian Ubuntu 24.04 (Noble) Minimal/IOT, Armbian Linux v6.1, Build: 2025-12-28
  * 役割: **推論専用（画像認識サーバ）**。設定は受け取るが、収集・保存・通知・UIは持たない。

* **ar-is22n / camServer（is22）**

  * 機材: GMKtec NucBox G5（N100系）
  * OS: Ubuntu Server 24.04.3 LTS（セットアップ済み）
  * 役割: **カメラ収集、インデックスDB、Drive保存、Web UI、通知、is21設定管理**を集約。

### 外部/周辺

* **RTSPカメラ ×30**（うち15台はVPN経由）
* **Google Drive API**（画像保存）
* **is10t**（ネットワーク巡回/情報取得済み。is22へ統合/投稿いずれも可）
* **外部Vision LLM（例: Gemini等）**

  * 原則は使わず、**unknown/要確認**のみ段階的に利用（低コスト運用）

---

## 1. 背景・目的

過去に「警察協力のため3週間分録画を人力で捜索」する事態が発生。
今後同様の事案に備え、以下を実現する：

1. 30台カメラの状況を **1分周期程度で俯瞰**できる（最低要件）
2. 「赤い服の人物」「深夜帯に2階」「犬/鹿」「浸水/倒木」「カメラ異常」等を **インデックス検索**できる
3. 通知は **観測事実のみ**に限定し、推測は“調査用メタデータ”として扱う
4. 画像は期限付きで破棄（プライバシー/容量対策）、ただし重要案件は隔離保管

---

## 2. 導入環境（ネットワーク/機器）

### VPNサブネット（IPsec Site-to-Site 常時接続）

* 192.168.3.0/24 : is10t, is21（東京都・本部）
* 192.168.125.0/24 : is22（京都府）、**is21もここへ設置可**
* 192.168.126.0/24 : デバイスなし

> 推奨: **is22とis21は同一拠点（同一サブネット）に設置**すると、画像送信の往復が安定し、VPN負荷も減る。
> ただし1分周期・リサイズ送信前提ならVPN越しでも成立する（最適化で対応）。

### カメラ（混在）

* TP-Link Tapo系、モデル混在
* RTSP疎通確認済み
* 参考（検証済み例）: Tapo C530WS

  * ONVIF: 2020/HTTP
  * 認証: WS-Security Digest (SHA1)
  * python-kasa Discovery OK（VPN越しUDPブロードキャストは不可）

---

## 3. システム全体像

### 機能分担（決定事項）

* is22（camServer）

  * スナップ収集（RTSP/必要ならONVIF）
  * **差分判定（軽量）**で「何もなし」を早期判定
  * is21へ推論リクエスト（リサイズ画像）
  * 結果を **MariaDB** にインデックス保存（「何もなし」も含む）
  * Driveへフル解像度保存（基本: 何もなしは保存しない）
  * Web UI提供（shadcn/ui）
  * 通知/日次要約/エスカレーション制御
  * is21のスキーマ（マークシートタグ）管理・配布

* is21（camimageEdge ai）

  * 受け取った画像（infer用）を推論
  * 返却は **enum選択＋数値**のみ（テキスト生成しない）
  * 推論以外（収集/保存/通知/DB/UI）は持たない

### データフロー（概略）

1. is22が各カメラからスナップ取得（60秒周期目安）
2. is22が infer画像へ正規化（例: 横640）＋差分判定
3. 差分が小さい → DBへ `no_event` を記録（is21へ送らない）
4. 差分あり → is21へ infer画像をPOST → enum結果を受領
5. `detected/unknown/severity`に応じて

   * DBへ保存
   * Driveへフル画像保存（ルールベース）
   * UI更新/通知
   * unknown時のみVision LLMへエスカレーション（クールダウン付き）

---

## 4. 機能要件（1〜16）対応表（要点）

1. is21推論結果返却のみ → ✅（APIサーバ＋推論のみ）
2. is22は収集/DB/Web/is21設定管理 → ✅
3. GoogleDrive API保存 → ✅（フル画像、条件保存）
4. is22がHTTPサーバでスナップ表示更新 → ✅
5. HDMI直表示（可能なら） → ✅（可能案提示、難しければ6へ）
6. 困難ならWeb表示 → ✅（基本はWeb）
7. UIは shadcn/ui デザインパターン → ✅
8. 管理設定は別ポート等 → ✅（例: 8081管理）
9. ローカルWebサーバ → ✅
10. 80/20レイアウト・狭幅はドロワー → ✅
11. 最大30分割・自動変更 → ✅
12. クリックでモーダル拡大→RTSP映像へ → ✅（RTSP→WebRTC/HLS変換）
13. 異常検知時は自動モーダル→一定時間で閉じる（設定） → ✅
14. is10t取得機能統合/投稿 → ✅（HTTP/MQTT連携）
15. インフォパネル上:ログ/検索/カレンダー、下:チャット（mobes） → ✅
16. is22稼働中サービス（Apache/MariaDB等）を前提に実装 → ✅

---

## 5. 非機能要件（設計方針）

* **目標周期**: 30台を概ね **1分/周**（各カメラ60秒更新）

  * 差分スキップにより平均負荷はさらに低下
* **耐障害**: カメラ個別失敗は全体停止に波及させない（タイムアウト・スキップ・バックオフ）
* **低コスト運用**: Vision LLMは **unknown時のみ**、かつ連続抑制
* **プライバシー/容量**:

  * 「何もなし」はDrive保存しない
  * 重要タグは隔離保管（保持延長）
  * DBの「何もなし」は期限/集計化でスリム化
* **セキュリティ**:

  * 管理UIは別ポート＋認証
  * 外部公開しない（ローカル/VPN内のみ）
  * Driveトークン/鍵の秘匿

---

## 6. is22（camServer）基本設計

### 6.1 サブシステム構成

1. **Camera Registry / Config**

* カメラ一覧・認証情報・RTSP URL・拠点属性（VPN/ローカル）
* is10tからの投稿を受け入れ可能（HTTP or MQTT）

2. **Snapshot Collector**

* 収集周期: デフォルト 60秒/台（重要カメラは短縮可能）
* タイムアウト: VPNカメラはやや長め（例: 3〜5秒）
* 失敗時: そのカメラのみ失敗記録、次周期へ

3. **Normalizer（infer生成）**

* 機種混在で解像度が一定でないため、**推論用画像を正規化**
* 推奨:

  * infer: 横640px（必要なら960）
  * 画角/アス比は維持（歪ませない）
  * 差分用: 320×180（または160×90）グレースケール

4. **Diff Gate（早期リターン）**

* 各カメラの前回infer（差分用縮小）と比較
* 小差分 → `no_event` でDB記録、推論スキップ
* ただし **強制推論**（例: 10回に1回 or 1時間に1回）を入れて見逃しを抑制

5. **Inference Dispatcher**

* 差分あり画像を is21へPOST
* is21が詰まったら「最新1枚だけ保持」キュー（カメラごと）でバックプレッシャ

6. **Indexer（MariaDB）**

* 推論結果JSON（生）＋検索用の正規化列を保存
* 「何もなし」も保存するが、保持期限を短くする運用を想定

7. **Drive Uploader**

* フル画像は **Driveへ保存**（原則: detected/unknown/重要タグのみ）
* 重要タグは隔離フォルダへ（保持延長）

8. **Web Backend**

* UI向けAPI（カメラ一覧、最新フレーム、イベント検索、ログ、ストリーム制御）

9. **Streaming Gateway（RTSP→ブラウザ）**

* ブラウザはRTSPを直接再生できないため、is22側で変換
* 推奨方式:

  * go2rtc / MediaMTX 等で **RTSP→WebRTC または HLS**
  * “モーダルが開いた時だけ”オンデマンドで開始、TTLで自動停止

10. **Notification / Scheduler**

* 即時通知（severity>=閾値）
* 19:00要約通知（WebHook）
* unknown時のVision LLMエスカレーション（連続抑制）

11. **Admin UI（別ポート）**

* カメラ設定、閾値、保持期限、隔離条件、is21スキーマ配布

---

## 7. is21（camimageEdge ai）基本設計

### 7.1 役割（厳守）

* HTTP APIで推論を受け、結果を返す
* **テキストは生成しない**（enumの選択結果のみ）
* 永続ストレージは持たない（モデル・設定ファイル程度はローカル保持可）

### 7.2 推論パイプライン（推奨）

1. 入力: is22から infer画像（640/960）
2. （任意）軽量品質判定: blur/dark など（タグ付け）
3. 検出: human/animal/vehicle/unknown 等
4. 属性（観測事実中心）:

   * 服色（top_color）: bbox内の簡易色判定（軽量・頑健）
   * 持ち物: backpack/bag/hat等（可能なら）
5. 出力: `primary_event + tags[] + confidence + severity + bbox[] + unknown_flag`

> ※ “人種/国籍/民族”の推定は行わない。
> 調査用に残す「観測寄りメタデータ」は **skin_tone（明暗）/hair_length/衣類形状/履物等**に限定し、通知には使わない（検索補助・低信頼タグとして扱う）。

---

## 8. タグ（マークシート）設計

### 8.1 基本方針

* is22が **schema（タグ定義）**を管理
* is21は schema_version を保持し、該当する tag_id だけ返す
* 通知用タグと調査用タグを分離する

### 8.2 タググループ（例）

* **event（主イベント）**: none / human / animal / vehicle / hazard_* / camera_issue / unknown
* **notify_safe（通知に出してよい観測事実）**:

  * count: single/multiple
  * top_color: red/blue/black/white/other
  * carry: backpack/bag/umbrella
  * behavior: loitering/running
  * hazard: smoke_like/water_present/fallen_tree_like
  * camera: blur/dark/glare/occluded/moved/offline
* **investigation_only（調査用・通知に出さない）**:

  * skin_tone: very_light/light/medium/dark/very_dark/unknown（“明暗”として）
  * hair_length: bald/short/medium/long/unknown
  * bottom_type: pants/skirt/dress/unknown
  * footwear_hint: heels/boots/sneakers/unknown
  * age_band_like: child_like/adult_like/older_adult_like/unknown（低信頼）

### 8.3 重要運用ルール

* `investigation_only` は **UIの調査モード（権限あり）だけで表示**
* 自動通知や自動判断（ロック等）には使わない
* confidenceが低いものは検索条件として“参考”扱い

---

## 9. データ設計（MariaDB）

### 9.1 テーブル方針

* フレーム粒度（毎スナップ）とイベント粒度（連続検知の束ね）を分離
* 「何もなし」フレームは短期保持、長期は集計に落とす

### 9.2 主要テーブル（案）

#### cameras

* camera_id（PK）
* name / location（例: 2F corridor）
* rtsp_url（秘匿: 別テーブル/暗号化推奨）
* network_zone（local/vpn）
* enabled, polling_interval_sec
* created_at, updated_at

#### frames

* frame_id（PK, UUID）
* camera_id（FK）
* ts
* detected (bool)
* primary_event（enum）
* tags_json（enum配列）
* severity, confidence
* diff_ratio, luma_delta（差分指標）
* infer_w, infer_h / full_w, full_h
* result_json（is21生JSON）
* drive_file_id（full保存時のみ）
* retention_class（normal/quarantine/case）
* indexes: (camera_id, ts), (detected, ts), (primary_event, ts)

#### events

* event_id（PK）
* camera_id（FK）
* start_ts, end_ts
* primary_event
* tags_summary_json
* best_frame_id（FK）
* drive_folder_id（必要なら）
* retention_class
* indexes: (camera_id, start_ts), (primary_event, start_ts)

#### drive_objects（任意）

* drive_file_id（PK）
* frame_id（FK）
* folder_type（normal/quarantine/case）
* created_at, expire_at

#### hourly_stats（長期用）

* camera_id, hour_ts
* total_frames, detected_frames, avg_diff, offline_count

### 9.3 保持（ところてん）

* frames（no_event）: 7〜14日で削除（要件に応じ調整）
* frames（detected/unknown）: 30日（通常）
* quarantine: 60日（重要タグ）
* case: 手動延長
* hourly_stats: 180日〜

---

## 10. Google Drive 保存設計

### 10.1 保存方針（決定事項）

* **Drive行きはフル画像**
* **「何もなし」はDrive保存しない**
* 重要イベントは隔離フォルダへ

### 10.2 フォルダ構成（例）

* `/camera-archive/YYYY/MM/DD/{camera_id}/...jpg`（通常）
* `/camera-quarantine/YYYY/MM/...`（隔離）
* `/case/{case_id}/...`（警察協力案件）

### 10.3 命名規則（例）

* `{camera_id}/{YYYY}/{MM}/{DD}/{ts}_{frame_id}.jpg`

---

## 11. API設計

### 11.1 is22 → is21 推論API（例）

* `POST http://is21:9000/v1/analyze`

  * body: multipart/form-data

    * camera_id, ts, schema_version
    * infer_image (jpeg)
    * optional: prev_infer_image（※将来拡張。基本はis22側で差分判定）
  * response: JSON（enumのみ）

レスポンス例（イメージ）:

```json
{
  "schema_version": "2025-12-28.1",
  "camera_id": "cam_2f_19",
  "ts": "2025-12-28T18:59:12+09:00",
  "detected": true,
  "primary_event": "human",
  "tags": ["count.single", "top_color.red", "behavior.loitering"],
  "confidence": 0.82,
  "severity": 2,
  "unknown_flag": false,
  "bbox": [[0.31,0.12,0.62,0.88]]
}
```

### 11.2 schema配布（is22 → is21）

* `PUT http://is21:9000/v1/schema`

  * schema_version, tags定義（JSON）
* `GET http://is21:9000/v1/schema`（確認用）
* `GET http://is21:9000/healthz`

### 11.3 is22 Web API（UI向け例）

* `/api/cameras`（一覧）
* `/api/cameras/{id}/latest`（最新スナップ）
* `/api/events/search`（条件検索: 期間/タグ/場所）
* `/api/events/{id}`（詳細）
* `/api/stream/start?camera_id=...`（WebRTC/HLS URL発行）
* `/api/admin/*`（別ポート、設定系）

---

## 12. Web UI設計（shadcn/ui）

### 12.1 画面レイアウト

* **メイン 80%**: カメラ分割グリッド（最大30）
* **右 20%**: インフォパネル

  * 画面幅が狭い場合はドロワー（スライド）に切替

### 12.2 メイン（分割グリッド）

* 表示台数に応じて自動でレイアウト変更（例: 2x2, 3x3, 5x6等）
* 各タイル:

  * 最新スナップ（HTTPで更新）
  * camera名/場所/状態アイコン
* クリック:

  * モーダル拡大 → RTSP映像（WebRTC/HLS）へ接続

### 12.3 異常時の自動モーダル（オプション）

* severity>=閾値のカメラは自動でモーダルを開く
* 指定秒で自動クローズ（設定可能）
* 連続トリガーは抑制（同一カメラはクールダウン）

### 12.4 インフォパネル

上段: ログ/検索

* 最新n件（no_eventは省略）
* 「もっと見る」→ モーダルで

  * カレンダー選択
  * インデックス検索結果
  * クリックで該当フレーム/イベントの画像モーダル

下段: AIチャット（mobes連携）

* 既存のチャットUIを再利用
* チャットでインデックス候補を提示→クリックで画像モーダル
* 画像モーダルがインフォパネルを潰さない（幅/レイヤ設計）

### 12.5 管理画面（別ポート）

* カメラ設定（URL、資格、場所、周期、VPN属性）
* 差分閾値、強制推論周期、通知閾値
* Drive保持/隔離ルール
* is21 schema管理（タグ追加/更新/バージョン）

---

## 13. 通知・エスカレーション設計

### 13.1 通知文は「観測事実のみ」

* is21はテキスト生成しない
* is22がテンプレで文章生成（非LLM）

  * 例: 「2階廊下で人物を検知（赤系上衣 / 1名 / 深夜帯）」
  * 断定的な属性推定は入れない

### 13.2 19:00 日次要約（WebHook）

* 当日分の events を集計
* 重要タグ/時間帯/場所でまとめて通知

### 13.3 Vision LLM（画像理解）への段階利用

* トリガー:

  * unknown_flag=true
  * 重大だが分類が曖昧（severity>=2 で confidence低い等）
* 連続抑制:

  * 同一カメラは10分クールダウン（例）
  * 同一イベントは代表フレームのみ
* 返却:

  * **自由文は“調査メモ”として保存**（任意）
  * ただし断定禁止・可能性表現で

---

## 14. RTSP映像表示（ブラウザ対応）

### 要点

* ブラウザはRTSPを直接再生不可
* is22で **RTSP→WebRTC/HLS** に変換し、モーダル表示する

### 方式（基本方針）

* オンデマンド:

  * モーダルが開いた時だけストリーム開始
  * 一定時間で停止（リソース節約）
* VPNカメラは遅延を考慮し接続タイムアウトを調整

---

## 15. HDMI表示（要件5/6）

### 方式A（可能なら）

* is22へ最小GUI（xorg + chromium kiosk）を導入し、ローカルWebを全画面表示

  * メリット: TVに直結して運用可能
  * デメリット: Ubuntu ServerにGUI追加の運用負荷

### 方式B（基本推奨）

* TV側（または別端末）から `http://is22/...` にアクセスして表示
* is22自体はサーバ専用で安定運用

---

## 16. 実装スタック（推奨）

### is22（NucBox）

* フロント: React（Next.js推奨） + shadcn/ui
* バック: Rust（axum/actix-web）推奨

  * MariaDB: sqlx
  * WebSocket/SSEでログ更新
* 既存Apache2:

  * 静的配信＋リバプロ（/apiをRustへ、/adminを別ポートへ）

### is21（OP5P）

* 推論サーバ: Python（FastAPIなど） or Rust（可能なら）
* RKNN等の推論ランタイム＋モデル（.rknn）
* systemdで常駐

---

## 17. 運用・監視

* systemdで各サービス常駐（collector / backend / streamer / uploader）
* ログ:

  * カメラ別成功/失敗
  * VPNカメラ遅延統計
  * 推論レイテンシ
  * Driveアップロード成功/失敗
* ヘルスチェック:

  * is21 `/healthz`
  * is22 `/healthz`
* バックアップ:

  * DB（events/frames）日次バックアップ
  * Driveは“保管庫”として、DBが索引の主

---

## 18. 受入基準（最小のDone）

1. 30台がグリッド表示され、クリックでモーダル→映像が見れる
2. 1分程度で最新画像が更新される（VPNカメラ含む）
3. 差分なしは推論スキップされ、DBにno_event記録される
4. 推論結果がDB検索できる（期間/カメラ/タグ）
5. detected/unknownのみDriveにフル画像が保存され、DBからリンク参照できる
6. 19:00要約通知が動作する
7. 管理UI（別ポート）でカメラ追加/閾値変更/schema更新ができる

---

# 付録A: 要件に合わせた「決定済みの設計原則」まとめ

* **is21は推論専用**（その他機能は持たない）
* **is22が収集/保存/通知/UI/設定を集中管理**
* **推論はリサイズ画像で実施**、証拠確認用に **Driveへフル画像**
* **差分が小さい場合は早期リターン**（no_event）
* **返却はenum選択**（マークシート）、テキスト生成はis22でテンプレ
* **unknown/重大のみ** Vision LLMへ（連続抑制）

---


