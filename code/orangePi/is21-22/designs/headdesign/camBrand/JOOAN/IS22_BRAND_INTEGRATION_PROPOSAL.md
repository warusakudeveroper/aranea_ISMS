# NVT/AltoBeam系カメラ IS22ブランド統合提案

## 作成日
2026-01-17（修正）

## 概要

JOOAN A6M-U（CAM720互換系）の実機検証を通じて判明したNVT/AltoBeam系カメラをIS22のカメラスキャン・自動登録システムに統合するための技術調査と提案。

**重要: ブランド名について**
- ONVIFで取得できるManufacturerは「**NVT**」（OEM元）
- MACアドレスOUIは「**AltoBeam Inc.**」（チップセットメーカー）
- 「JOOAN」というブランド名は技術的に自動識別不可能
- IS22では**ONVIFのManufacturer値（NVT）をそのまま使用**する方針

---

## 1. 実機検証で判明した情報

### 1.1 検証対象
- 商品ブランド: JOOAN
- モデル: A6M-U
- 管理アプリ: CAM720 (WEIZHI社)

### 1.2 ONVIF GetDeviceInformation結果

| 項目 | 取得値 |
|------|--------|
| Manufacturer | **NVT** |
| Model | JA-A1A |
| FirmwareVersion | 05.02.31.69 |
| SerialNumber | (空) |
| HardwareId | IPC |

### 1.3 MACアドレス/OUI

| 項目 | 値 |
|------|-----|
| MAC | 48:D0:1C:1F:2F:6D |
| OUIプレフィックス | 48:D0:1C |
| OUIベンダー | **AltoBeam Inc.**（北京） |

### 1.4 ポート構成

| ポート | 用途 |
|--------|------|
| 80 | HTTP (Web UI) |
| 443 | HTTPS |
| 554 | RTSP |
| **8899** | **ONVIF** ★非標準 |

### 1.5 RTSP情報

| 項目 | 値 |
|------|-----|
| メインストリーム | `/live/ch00_0` (2304x1296) |
| サブストリーム | `/live/ch00_1` (640x360) |
| ユーザー名 | `admin`（固定） |
| 認証方式 | Basic |

---

## 2. TP-link vs NVT(JOOAN) 技術比較

### 2.1 識別可能性の違い

| 項目 | TP-link (Tapo) | NVT (JOOAN実機) |
|------|----------------|-----------------|
| OUI登録名 | **TP-Link Corporation** | AltoBeam Inc. |
| ONVIF Manufacturer | **tp-link** | NVT |
| ブランド自動識別 | ✅ **可能** | ❌ **不可能** |

**TP-linkが自動識別可能な理由:**
- OUIが「TP-Link」として登録されている
- ONVIFでも「tp-link」が返る
- 両方が一致するため確実に識別可能

**NVT(JOOAN)が自動識別不可能な理由:**
- OUIは「AltoBeam」（チップメーカー）
- ONVIFは「NVT」（OEM元）
- 「JOOAN」という商品ブランド情報が技術的に取得不可

### 2.2 ポート・RTSPパスの違い

| 項目 | TP-link (Tapo) | NVT (JOOAN実機) |
|------|----------------|-----------------|
| ONVIFポート | 2020 | **8899** |
| RTSPメイン | `/stream1` | `/live/ch00_0` |
| RTSPサブ | `/stream2` | `/live/ch00_1` |

---

## 3. ブランド登録方針

### 3.1 基本方針

**ONVIFのManufacturer値をそのまま使用**

```
検出時: AltoBeam(OUI) → ONVIFプローブ → Manufacturer=NVT → "NVT"として登録
```

- JOOANと明記しない（技術的に識別不可能なため）
- NVTとして登録されることでOEM元が明確になる
- 同じNVT OEMを使う他ブランドも同様に扱える

### 3.2 OUI活用による効率化

OUI（AltoBeam）を活用してカメラ検出を効率化:

```
48:D0:1C (AltoBeam) 検出
  → カメラ可能性スコア加算
  → ポート8899を優先スキャン
  → ONVIFプローブ実行
  → Manufacturer=NVT として登録
```

**メリット:**
- AltoBeamチップ搭載機器を早期にカメラ候補として検出
- 非標準ONVIFポート(8899)への対応が可能
- スキャン効率の向上

---

## 4. 実装内容

### 4.1 DBマイグレーション

```sql
-- ========================================
-- NVTブランド追加（ONVIFのManufacturer値）
-- ========================================
INSERT INTO camera_brands (name, display_name, category, is_builtin) VALUES
('NVT', 'NVT (OEM)', 'consumer', TRUE);

-- ========================================
-- AltoBeam OUI追加（カメラ検出効率化用）
-- ========================================
-- 注意: brand_idはNVTに紐付け（ONVIFで最終的にNVTと識別されるため）
INSERT INTO oui_entries (oui_prefix, brand_id, description, score_bonus, status, verification_source, is_builtin)
SELECT '48:D0:1C', b.id, 'AltoBeam Inc. - NVT OEM系カメラチップ', 20, 'confirmed', '実機検証 2026-01-17', TRUE
FROM camera_brands b WHERE b.name = 'NVT';

-- ========================================
-- NVT用RTSPテンプレート追加
-- ========================================
INSERT INTO rtsp_templates (brand_id, name, main_path, sub_path, default_port, is_default, priority, is_builtin)
SELECT b.id, 'NVT Standard (CAM720互換)', '/live/ch00_0', '/live/ch00_1', 554, TRUE, 10, TRUE
FROM camera_brands b WHERE b.name = 'NVT';

-- ========================================
-- generic_rtsp_pathsにも追加（フォールバック用）
-- ========================================
INSERT INTO generic_rtsp_paths (main_path, sub_path, description, priority, is_enabled) VALUES
('/live/ch00_0', '/live/ch00_1', 'NVT/CAM720互換 (JOOAN等)', 45, TRUE);
```

### 4.2 コード変更

#### 4.2.1 スキャンポート追加

```rust
// src/ipcam_scan/utils.rs
pub fn default_ports() -> Vec<u16> {
    vec![554, 2020, 8899, 80, 443, 8000, 8080, 8443, 8554]
    //         ^^^^ NVT系ONVIF用に追加
}
```

#### 4.2.2 ベンダー判定でのヒント追加（オプション）

```rust
// src/ipcam_scan/utils.rs: generate_detection_reason()
// AltoBeam検出時のユーザーメッセージ改善
if vendor_upper.contains("ALTOBEAM") {
    if evidence.open_ports.contains(&554) || evidence.open_ports.contains(&8899) {
        (
            DeviceType::CameraLikely,
            "NVT系カメラの可能性あり (ONVIFポート8899を確認)".to_string(),
            SuggestedAction::SetCredentials,
        )
    }
}
```

### 4.3 ONVIFプローブのポート対応

現在のIS22はONVIFポート2020を優先的にプローブしている。
NVT系対応のため、ポート8899も試行対象に含める:

```rust
// ONVIFプローブ時のポート試行順序
let onvif_ports = [2020, 8899, 80];
```

---

## 5. カメラスキャンフロー（NVT系）

```
┌─────────────────────────────────────────────────────────────┐
│ 1. ARPスキャン                                              │
│    MAC: 48:D0:1C:XX:XX:XX 検出                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. OUIルックアップ                                          │
│    48:D0:1C → AltoBeam Inc.                                │
│    → スコア+20、カメラ可能性フラグON                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. ポートスキャン                                           │
│    554 (RTSP) → Open                                       │
│    8899 (ONVIF) → Open ★非標準ポート                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 4. ONVIFプローブ (ポート8899)                               │
│    GetDeviceInformation → Manufacturer: "NVT"              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│ 5. 登録                                                     │
│    manufacturer: "NVT"                                     │
│    model: "JA-A1A"                                         │
│    rtsp_main: /live/ch00_0                                 │
│    rtsp_sub: /live/ch00_1                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. 実装優先度

| 優先度 | 項目 | 工数 | 備考 |
|--------|------|------|------|
| P1 | DBマイグレーション（NVTブランド、OUI、RTSPテンプレート） | 小 | 必須 |
| P2 | default_ports()に8899追加 | 小 | 必須 |
| P3 | ONVIFプローブでポート8899試行 | 小 | 必須 |
| P4 | generic_rtsp_pathsに追加 | 小 | フォールバック用 |
| P5 | AltoBeam検出時のUIメッセージ改善 | 小 | オプション |

**合計工数: 0.5-1日程度**

---

## 7. 今後の拡張

### 7.1 NVT OEM製品の追加

NVTをOEMとして使用する他ブランドが見つかった場合:
- 同じ「NVT」ブランドとして登録される
- RTSPテンプレートは共通で使用可能
- OUIが異なる場合のみoui_entriesに追加

### 7.2 AltoBeamチップの他用途

AltoBeamはWi-Fi/BLEチップメーカーのため、カメラ以外のIoT機器にも使用される可能性:
- OUIのstatusを`confirmed`で登録するが、非カメラ機器の誤検出に注意
- ポート554/8899が開いている場合のみカメラ判定

---

## 8. 結論

1. **ブランド名**: ONVIFのManufacturer値「**NVT**」をそのまま使用
2. **OUI活用**: AltoBeam(48:D0:1C)でカメラ検出効率化
3. **ポート追加**: 8899をスキャン・ONVIFプローブ対象に追加
4. **RTSPテンプレート**: `/live/ch00_0`, `/live/ch00_1`をNVT向けに登録
5. **JOOANの扱い**: 技術的に識別不可能なため明記しない

**この方針により:**
- IS22の既存フロー（ONVIFベースの登録）と整合
- OUIによるスキャン効率化を維持
- 技術的に正確な情報（NVT）で登録
- 将来のNVT OEM製品にも対応可能
