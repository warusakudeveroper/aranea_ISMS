# is20s 設計概要

## 概要

is20s は Orange Pi Zero3 上で動作するネットワーク監視・ローカルリレーデバイス。
ER605ルーターのミラーポートからパケットキャプチャし、部屋単位の通信先を可視化する。

## 主要機能

1. **パケットキャプチャ**: tsharkでSYN-onlyモードのキャプチャ
2. **サービス識別**: DNS/SNI/ASNからアクセス先サービスを特定
3. **脅威検知**: StevenBlack/hosts、Tor出口ノードリストによる脅威インテリジェンス
4. **ローカルリレー**: ESP32デバイスからのデータ中継
5. **Web UI**: リアルタイムモニタリング

## 技術スタック

- **OS**: Armbian 25.5.1 (Ubuntu Noble)
- **言語**: Python 3.12
- **フレームワーク**: FastAPI
- **パケットキャプチャ**: tshark (Wireshark)
- **ASN検索**: Team Cymru DNS

## 詳細

詳細設計は [DESIGN_detail.md](./DESIGN_detail.md) を参照。
