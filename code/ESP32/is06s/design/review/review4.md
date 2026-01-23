把握。`efffebc` の内容は確認できていて、**今回の“暫定修正”は狙いどおり**です。

* `is06s.ino` の `stateReporter.update()` を停止（HTTPS/TLS を完全に踏まない）
* `StateReporterIs06s::begin()` 側で「初回 heartbeat 即送信」を `lastHeartbeatTime_ = millis()` で遅延（ただし今回は update 自体を止めてるので副次的）

この2点で、調査メモに書いてる **「StateReporter ON（遅延あり）でも約2分後に落ちる」＝TLS経路が踏まれる限り地雷**という結論と整合します。

一方で、あなたが挙げた **検証観点 1〜3**は「暫定修正が正しいか」を短時間で判定できる、良いセットです。

---

## 検証の“見るべきログ”だけ先に固定（3分テストの合否が明確になる）

### 1) 3分以上安定動作

合格条件（最低限）：

* 起動後、`rst:` 系が一切出ない
* “例の” `Done: 1 success`（StateReporter由来）も出ない（update を止めてるので出ないのが正しい）

補助ログ（あると強い）：

* `esp_reset_reason()` を setup 冒頭で1回出す（※これはクラッシュ時に「何が起きたか」を次の周回で確定できる）

---

### 2) MQTT publish/subscribe 正常

このFWは「onConnected 内 publish 禁止」を回避するため `mqttJustConnected` フラグで loop 側 publish に寄せてるので、ここが見れればOKです：

* 接続後に `MQTT: Connected!` → `Subscribed...` → `Publishing initial states...` の順に出る
* その後、`aranea/{tid}/{lacisId}/command` に `{"cmd":"getState"}` か `{"cmd":"set","ch":1,"state":1}` を投げて

  * `MQTT: Received...`
  * `MQTT: ACK sent [ok]`
  * 状態 topic に publish
    が揃えば合格（publish は `mqttStateTopic` / `mqttAckTopic`）

---

### 3) Web UI (HTTP) 正常

合格条件：

* `HTTP: Web UI available at http://<ip>/` が出る
* `/` が表示できる
* PIN系 API が呼べる（たとえば CH の状態取得/設定が成功）

---

## 次の一手（暫定修正が合格したら、恒久対策の優先度）

暫定が通った時点で **「落ちる経路は TLS/HTTPS 通信」**がほぼ確定なので、恒久はこの順が現実的です：

1. **HTTP中継サーバ（推奨）**
   ESP32側は “平文HTTP” or “LAN内HTTP” で投げる → サーバ側が Cloud Functions へ HTTPS
   （ESP32からTLSを排除できるので、再発率が劇的に下がる）

2. `setInsecure()`
   “動く可能性”は高いけど、プロダクトとしてのセキュリティ設計は要整理

3. ESP-IDF直
   工数重い。最終的にやるなら「HTTP/TLSスタックだけIDF寄せ」みたいな段階案がいい

---

## ひとつだけ注意点

`StateReporterIs06s::begin()` の `lastHeartbeatTime_ = millis()` は「初回即送信回避」には効くんですが、調査メモ通り **“遅延しても2分後に落ちる”**なら根治ではないです。
なので、**今回の「update完全停止」＝暫定として正解**です。

---

もし、検証後ログ（MQTT接続〜コマンド投入〜ACKまで）を貼ってくれれば、「どこが詰まりやすいか（topic/認証/cic/tid/subscribe timing）」まで一発で見ます。
