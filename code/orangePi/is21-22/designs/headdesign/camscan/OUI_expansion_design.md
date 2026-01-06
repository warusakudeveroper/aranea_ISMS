# OUI判定リスト拡充 設計ドキュメント

## 1. 概要

### 1.1 目的
カメラスキャン機能のOUI（Organizationally Unique Identifier）判定リストを拡充し、より多くのカメラベンダーを自動識別可能にする。

### 1.2 対象ファイル
`src/ipcam_scan/scanner/oui_data.rs`

### 1.3 現状分析
現在のOUI_CAMERA_VENDORSには23エントリが登録済み：
- TP-Link / Tapo: 13エントリ
- Google / Nest: 3エントリ
- Hikvision: 3エントリ
- Dahua: 2エントリ
- Axis: 2エントリ

### 1.4 追加対象ベンダー（Camscan_designers_review.mdより）
1. Anker Eufy
2. Ring
3. SwitchBot
4. EZVIZ
5. F.R.C（エフアールシー）
6. I.O.DATA
7. Panasonic / i-PRO
8. Arlo
9. Anpviz
10. Reolink
11. Amcrest

---

## 2. 調査結果

### 2.1 IEEE OUIデータベース調査結果

| ベンダー | OUIプレフィックス | 調査ステータス | 備考 |
|---------|-----------------|---------------|------|
| Ring LLC | 12件確認 | 完了 | Amazon傘下 |
| EZVIZ | 13件確認 | 完了 | Hangzhou Ezviz Software Co.,Ltd. |
| Reolink | 1件確認 | 完了 | Reolink Innovation Limited |
| Amcrest | 4件確認 | 完了 | Amcrest Technologies |
| Arlo | 3件確認 | 完了 | Arlo Technology |
| I.O.DATA | 3件確認 | 完了 | I-O DATA DEVICE,INC. |
| Panasonic | 37件確認 | 完了 | 主要なものを選定 |
| SwitchBot | 1件確認 | 完了 | Shenzhen Intellirocks Tech. |
| Anker Eufy | 1件確認 | 部分的 | ANKER-EAST、Eufyは別OEM使用 |
| F.R.C | 未確認 | 調査継続 | 日本国内メーカー、OUI未特定 |
| Anpviz | 未確認 | 調査継続 | OEMの可能性（Dahua/Hikvision） |

### 2.2 収集済みOUIプレフィックス

#### Ring LLC（12件）
```
00:B4:63, 18:7F:88, 24:2B:D6, 34:3E:A4, 54:E0:19, 5C:47:5E,
64:9A:63, 90:48:6C, 9C:76:13, AC:9F:C3, C4:DB:AD, CC:3B:FB
```

#### EZVIZ / Hangzhou Ezviz Software（13件）
```
0C:A6:4C, 20:BB:BC, 34:C6:DD, 54:D6:0D, 58:8F:CF, 64:24:4D,
64:F2:FB, 78:A6:A0, 78:C1:AE, 94:EC:13, AC:1C:26, EC:97:E0, F4:70:18
```

#### Reolink（1件）
```
EC:71:DB
```

#### Amcrest（4件）
```
00:65:1E, 9C:8E:CD, A0:60:32, 34:46:63 (MA-M block)
```

#### Arlo Technology（3件）
```
48:62:64, A4:11:62, FC:9C:98
```

#### I.O.DATA DEVICE（3件）
```
00:A0:B0, 34:76:C5, 50:41:B9
```

#### Panasonic（主要10件選定）
```
00:80:45, 00:80:F0, 00:1C:B3, 04:20:9A, 08:00:23,
20:C6:EB, 34:F6:D2, 54:CD:10, 80:C7:55, EC:65:CC
```
注：Panasonicは37件以上のOUIを保有。カメラ関連と思われる主要なものを選定。

#### SwitchBot / Shenzhen Intellirocks（1件）
```
D4:AD:FC
```

#### Anker Eufy（暫定1件）
```
70:B3:D5 (ANKER-EAST)
```
注：Eufyカメラは異なるOEMメーカーを使用している可能性あり。実機確認推奨。

---

## 3. 実装計画

### 3.1 追加エントリ数
- 確定追加: 37件（Ring 12 + EZVIZ 13 + Reolink 1 + Amcrest 4 + Arlo 3 + I.O.DATA 3 + SwitchBot 1）
- 追加候補: 11件（Panasonic 10 + Anker/Eufy 1）
- 合計: 48件の追加（既存23件 + 追加48件 = 計71件）

### 3.2 スコア設計
全ベンダーに対して統一スコア20を付与（既存と同様）。
```rust
const SCORE_OUI_BONUS: u32 = 20;
```

### 3.3 カテゴリ分類（将来的な表示用）
| カテゴリ | ベンダー |
|---------|---------|
| 業務用カメラ | Hikvision, Dahua, Axis, Panasonic/i-PRO |
| 家庭用カメラ | TP-Link/Tapo, Ring, EZVIZ, Reolink, Amcrest, Arlo, Eufy |
| スマートホーム | Google/Nest, SwitchBot |
| NAS/周辺機器 | I.O.DATA |

---

## 4. コード変更

### 4.1 変更対象
`src/ipcam_scan/scanner/oui_data.rs` の `OUI_CAMERA_VENDORS` 定数

### 4.2 変更内容

```rust
/// OUI prefixes for camera vendors
pub const OUI_CAMERA_VENDORS: &[(&str, &str, u32)] = &[
    // ========== TP-Link / Tapo (既存) ==========
    ("70:5A:0F", "TP-LINK", 20),
    ("54:AF:97", "TP-LINK", 20),
    ("B0:A7:B9", "TP-LINK", 20),
    ("6C:5A:B0", "TP-LINK", 20),
    ("6C:C8:40", "TP-LINK", 20),
    ("08:A6:F7", "TP-LINK", 20),
    ("78:20:51", "TP-LINK", 20),
    ("BC:07:1D", "TP-LINK", 20),
    ("18:D6:C7", "TP-LINK", 20),
    ("3C:84:6A", "TP-LINK", 20),
    ("D8:07:B6", "TP-LINK", 20),
    ("D8:44:89", "TP-LINK", 20),
    ("9C:53:22", "TP-LINK", 20),

    // ========== Google / Nest (既存) ==========
    ("F4:F5:D8", "Google", 20),
    ("30:FD:38", "Google", 20),
    ("3C:22:FB", "Google", 20),

    // ========== Hikvision (既存) ==========
    ("A8:42:A1", "Hikvision", 20),
    ("C0:56:E3", "Hikvision", 20),
    ("44:19:B6", "Hikvision", 20),

    // ========== Dahua (既存) ==========
    ("3C:EF:8C", "Dahua", 20),
    ("4C:11:BF", "Dahua", 20),

    // ========== Axis (既存) ==========
    ("00:40:8C", "Axis", 20),
    ("AC:CC:8E", "Axis", 20),

    // ========== Ring (新規 12件) ==========
    ("00:B4:63", "Ring", 20),
    ("18:7F:88", "Ring", 20),
    ("24:2B:D6", "Ring", 20),
    ("34:3E:A4", "Ring", 20),
    ("54:E0:19", "Ring", 20),
    ("5C:47:5E", "Ring", 20),
    ("64:9A:63", "Ring", 20),
    ("90:48:6C", "Ring", 20),
    ("9C:76:13", "Ring", 20),
    ("AC:9F:C3", "Ring", 20),
    ("C4:DB:AD", "Ring", 20),
    ("CC:3B:FB", "Ring", 20),

    // ========== EZVIZ (新規 13件) ==========
    ("0C:A6:4C", "EZVIZ", 20),
    ("20:BB:BC", "EZVIZ", 20),
    ("34:C6:DD", "EZVIZ", 20),
    ("54:D6:0D", "EZVIZ", 20),
    ("58:8F:CF", "EZVIZ", 20),
    ("64:24:4D", "EZVIZ", 20),
    ("64:F2:FB", "EZVIZ", 20),
    ("78:A6:A0", "EZVIZ", 20),
    ("78:C1:AE", "EZVIZ", 20),
    ("94:EC:13", "EZVIZ", 20),
    ("AC:1C:26", "EZVIZ", 20),
    ("EC:97:E0", "EZVIZ", 20),
    ("F4:70:18", "EZVIZ", 20),

    // ========== Reolink (新規 1件) ==========
    ("EC:71:DB", "Reolink", 20),

    // ========== Amcrest (新規 4件) ==========
    ("00:65:1E", "Amcrest", 20),
    ("34:46:63", "Amcrest", 20),  // MA-M block (first 3 bytes)
    ("9C:8E:CD", "Amcrest", 20),
    ("A0:60:32", "Amcrest", 20),

    // ========== Arlo (新規 3件) ==========
    ("48:62:64", "Arlo", 20),
    ("A4:11:62", "Arlo", 20),
    ("FC:9C:98", "Arlo", 20),

    // ========== I.O.DATA (新規 3件) ==========
    ("00:A0:B0", "I-O-DATA", 20),
    ("34:76:C5", "I-O-DATA", 20),
    ("50:41:B9", "I-O-DATA", 20),

    // ========== Panasonic / i-PRO (新規 10件) ==========
    ("00:80:45", "Panasonic", 20),
    ("00:80:F0", "Panasonic", 20),
    ("00:1C:B3", "Panasonic", 20),
    ("04:20:9A", "Panasonic", 20),
    ("08:00:23", "Panasonic", 20),
    ("20:C6:EB", "Panasonic", 20),
    ("34:F6:D2", "Panasonic", 20),
    ("54:CD:10", "Panasonic", 20),
    ("80:C7:55", "Panasonic", 20),
    ("EC:65:CC", "Panasonic", 20),

    // ========== SwitchBot (新規 1件) ==========
    ("D4:AD:FC", "SwitchBot", 20),

    // ========== Anker/Eufy (新規 1件) ==========
    ("70:B3:D5", "Anker/Eufy", 20),
];
```

---

## 5. テスト計画

### 5.1 単体テスト
1. `lookup_oui()` 関数のテスト
   - 既存ベンダーのOUI判定が維持されること
   - 新規追加ベンダーのOUI判定が正しく動作すること
   - 未登録OUIがNoneを返すこと

### 5.2 結合テスト
1. カメラスキャン実行
   - 新規ベンダーのカメラがスキャン結果に正しいベンダー名で表示されること
   - スコア計算が正しく行われること（OUIボーナス+20）

### 5.3 UIテスト
1. フロントエンドでのスキャン結果表示
   - ベンダー名が正しく表示されること
   - 日本語表示に問題がないこと

---

## 6. 未解決事項

### 6.1 F.R.C（エフアールシー）
- 日本国内メーカーだがIEEE OUIデータベースでの登録が確認できず
- 実機のMACアドレスを確認して追加する必要あり

### 6.2 Anpviz
- OEMメーカーの可能性（Hikvision/Dahua系）
- 実機確認が必要

### 6.3 Eufy
- ANKER-EASTのOUIを暫定登録
- 実機のMACアドレスで異なるOUIが使用されている可能性あり

---

## 7. MECE確認

- [x] 既存ベンダーのエントリは維持される
- [x] 新規ベンダーは重複なく追加される
- [x] スコア設計は既存と統一されている
- [x] 未確認ベンダーは明確に識別され、将来追加の道筋が示されている

---

## 8. 参考資料

### 8.1 OUIデータソース
- maclookup.app
- IEEE OUI Public Database (standards.ieee.org)
- adminsub.net/mac-address-finder

### 8.2 関連ドキュメント
- Camscan_designers_review.md（デザイナーレビュー）
- The_golden_rules.md（開発ルール）

---

## 9. 承認・実装フロー

1. [ ] 設計レビュー完了
2. [ ] GitHub Issue登録
3. [ ] 実装前コミット
4. [ ] コード実装
5. [ ] 単体テスト実行
6. [ ] 結合テスト実行
7. [ ] UIテスト実行
8. [ ] 完了報告

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
