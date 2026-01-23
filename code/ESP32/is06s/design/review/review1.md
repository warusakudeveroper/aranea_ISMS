• 一次結論（クラッシュ要因の最有力候補）

  - MQTT無効化後もTLSハンドシェイク系のログ（「e: 1 success」）で即リブートしている症状は、
    StateReporterIs06s::update()が起動直後にHTTPS POSTを実行する設計がトリガーの可能性が高い。デフォルトでcloud_url
    がHTTPSに設定され、ハートビート間隔60秒にもかかわらず初回は即時送信されるため、MQTTを切ってもTLSセッションが毎起
    動で走る（code/ESP32/is06s/StateReporterIs06s.cpp）。
  - TLSクラッシュは先のMQTT WSSと同系統の事象と推測。HTTPClientによるhttps://asia-northeast1-
    mobesorder.cloudfunctions.net/deviceStateReportへの初回POSTが原因か確認が必要。

  追加で見つけたリスク/仕様

  - 初回ハートビート送信が即時で、バックオフまでに少なくとも1回のHTTPS通信が必ず走る（StateReporterIs06s::update →
    sendHeartbeat → sendStateReport）。WiFi接続直後かつNTP同期直後にTLS負荷がかかり、2.5秒リセットと一致しやすい。
  - cloudUrl/tid/cicがNVSに残っていると、MQTT無効でもクラウドPOSTは有効のまま。MQTTを#if 0してもクラッシュが継続する
    説明と整合。

  切り分けの優先ステップ（再現確認用）

  1. stateReporter.update()を一時無効化、またはstateReporter.setHeartbeatInterval(0)で送信停止したビルドを試す。これ
     で安定すればTLS起因が確定。
  2. cloud_urlを空文字に設定し、クラウドPOSTを明示的に無効化して再テスト。
  3. 上記が安定する場合、TLSスタック/証明書/HTTPClient側の問題として、WiFiClientSecureにsetInsecure()を適用するか、
     HTTPをローカル中継（リレーURLのHTTP化）に切り替える案を検討。
  4. さらなる切り分けとして、Phase 1手順どおりStateReporter単体ON→OFFで再現性確認（Is06PinManager/HttpManagerを順次
     有効化）。

  追加ログ/計測の提案

  - 起動直後のloop()冒頭でesp_reset_reason()を出力し、リセット種別を確認（0x1かWDT/abortか判別）。
  - StateReporterIs06s::sendStateReport入口にESP.getFreeHeap()/ESP.getMinFreeHeap()を出力し、TLS前後のヒープを確認。
  - TLSハンドシェイク中のクラッシュなら、HTTPClientをWiFiClientSecure client; client.setInsecure();
    http.begin(client, url);に変更する暫定パッチで改善するかを確認（恒久策は証明書設定）。

  現時点の最短トライアル案

  - ビルドスイッチでSTATE_REPORTER_ENABLEDを導入し、OFFビルドを一度実機で確認（MQTT無効のまま）。
  - OFFで安定するなら、HTTP/TLS経路の改善を優先（上記setInsecureまたはHTTPエンドポイントへのリレー）。

  補足: 依頼どおりProductCodeはコード上0200に統一済み（code/ESP32/is06s/is06s.ino定数およびStateReporter payload）。