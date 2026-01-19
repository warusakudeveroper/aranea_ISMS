# Tapo カメラ ストリームアーキテクチャ解析

## 調査日
2026-01-17

## 結論サマリー

| 項目 | 値 | 根拠 |
|------|-----|------|
| ネイティブストリーム数 | **2本** (stream1/stream2) | 公式FAQ |
| 最大同時セッション | **4** (2ストリーム × 各2接続) | 公式FAQ + コミュニティ解釈 |
| 実運用上の安全値 | **3以下** | 公式「通常3台まで」 |
| PTZモデル推奨 | **1セッション** | 放熱・処理負荷問題 |
| 固定モデル推奨 | **2セッション** | 実測で安定 |

---

## 1. ストリーム構成（公式情報）

### 基本構成
```
stream1 (HQ/Main): 高画質ストリーム
stream2 (LQ/Sub):  低画質ストリーム
```

- RTSP URL: `rtsp://user:pass@IP:554/stream1` / `/stream2`
- ONVIFポート: 2020
- Profile: ONVIF Profile S（2-way audio非対応）

### 「最大4ストリーム」の解釈

公式FAQの「最大4ストリーム（2 main + 2 sub）」は：

```
実態: 2本のエンコード × 各2接続 = 合計4セッション
```

| ストリーム | 接続1 | 接続2 |
|-----------|-------|-------|
| stream1 (HQ) | ✅ | ✅ |
| stream2 (LQ) | ✅ | ✅ |

**注意**: 別の公式FAQでは「ローカル同時視聴は通常3台まで」とも記載。

---

## 2. PTZ vs 固定カメラの性能差

### スペック比較

| モデル | 種別 | 解像度 | FPS | PTZ | 備考 |
|--------|------|--------|-----|-----|------|
| C200 | PTZ | 1080p | 15fps | ✅ | 放熱問題報告あり |
| C210 | PTZ | 2K (3MP) | 15fps | ✅ | 放熱問題報告あり |
| C100 | 固定 | 1080p | 15fps | ❌ | 比較的安定 |
| C110 | 固定 | 2K (3MP) | 15fps | ❌ | 2並列OK確認済み |

### PTZモデルが不利な理由

1. **モーター制御処理**: 常時待機でCPUリソース消費
2. **放熱問題**: FrigateコミュニティでIC過熱報告
   > "These cameras don't have the heat dissipation for continuous streaming to an nvr"
3. **無線負荷**: PTZ制御コマンドもWiFi帯域を消費
4. **付加処理**: 位置センサー・ジャイロ処理

### 実測データ（IS22環境）

| テスト | C110 (固定) | C200 (PTZ) |
|--------|-------------|------------|
| 2並列接続 | ✅ 100% | ❌ 0% |
| Main+Sub同時 | ✅ 100% | ❌ 0% |
| 急速再接続 | ✅ 100% | ✅ 100% |

---

## 3. Access Absorber設計指針

### パラメータ定義

```rust
struct TapoStreamConfig {
    native_streams: u8,           // = 2 (stream1/2)
    max_upstream_sessions: u8,    // カメラへの接続数
    max_upstream_safe: u8,        // PTZ/高温時のフェイルセーフ
    max_downstream_clients: u8,   // Absorberからの再配信（無制限可）
}
```

### 推奨設定

| モデル分類 | max_upstream | require_exclusive | 理由 |
|-----------|--------------|-------------------|------|
| tapo_ptz | 1 | TRUE | 放熱・処理負荷 |
| tapo_fixed | 2 | FALSE | 実測で安定 |

### 実装のコツ

1. **カメラへはAbsorberだけが接続**
   - Frigate/HA/他NVRはAbsorberの再配信へ
   - カメラ直接接続を禁止

2. **2ストリーム運用**
   ```
   Absorber → stream1 (record用)
   Absorber → stream2 (detect/preview用)
   ```

3. **フェイルセーフ**
   - 接続失敗3回でmax_upstream_sessions = 1に自動縮退
   - 温度監視可能なら高温時に縮退

---

## 4. 参考リンク

### 公式
- [Tapo RTSP/ONVIF FAQ](https://www.tapo.com/us/faq/724/)
- [TP-Link RTSP設定ガイド](https://www.tp-link.com/us/support/faq/2680/)

### コミュニティ
- [Frigate - Tapo C200/C210 Discussion](https://github.com/blakeblackshear/frigate/discussions/20746)
- [go2rtc - Tapo Issues](https://github.com/AlexxIT/go2rtc/issues/398)
- [TP-Link Community - Device Limit](https://community.tp-link.com/en/smart-home/forum/topic/720802)

---

## 5. 今後の検証課題

- [ ] C210（PTZ 3MP）のストレステスト
- [ ] ファームウェアバージョンによる挙動差の確認
- [ ] 高温環境（夏季）での再テスト
- [ ] SD録画併用時の接続数変化
