# IS22 カメラプリセット設計書

## 改訂履歴
| 日付 | バージョン | 変更内容 |
|------|----------|----------|
| 2026-01-02 | 1.0.0 | 初版作成 |
| 2026-01-02 | 1.1.0 | プリセット説明・共有設定・データ保存仕様追加 |

---

## 概要
CameraDetailModalから選択可能なカメラプリセットを定義。
プリセットにより検出パラメータと戻り値構造を正規化し、ログの一貫性を確保する。

### 設計原則
1. **説明の可視化**: プリセット選択時に仕様・検出傾向・推奨場所を表示
2. **トレーサビリティ**: preset_idをリクエスト/レスポンス/DB/BQ/LLMすべてに含める
3. **共有定義**: is22とis21が同一のプリセット定義ファイルを参照
4. **チューニング対応**: プリセット定義はバージョン管理し、後のチューニングで活用

---

## 1. プリセット一覧

| ID | 名称 | 主用途 | 優先検出 |
|----|------|--------|----------|
| `person_priority` | 人物優先 | 人物特徴詳細取得 | 人物 |
| `balanced` | バランス | 汎用監視 | 人物・車両・動物 |
| `high_detection` | 高検出 | 見逃し最小化 | 全オブジェクト（低閾値） |
| `parking` | 駐車場 | 車両管理 | 車両・ナンバー |
| `corridor` | 施設内通路 | 通行監視・滞在検知 | 人物・滞在 |
| `restricted` | 警戒区域 | 侵入検知 | 人物（高警戒） |
| `entrance` | 玄関・フロント | 来客対応 | 人物・荷物 |
| `dining` | 飲食ホール | 客席監視 | 人物・グループ |
| `kitchen` | 厨房 | 衛生・安全 | 人物・制服・衛生装備 |
| `campsite` | キャンプ場 | 野外監視 | 人物・車両・動物 |
| `outdoor` | 施設屋外 | 外周監視 | 人物・車両・動物 |
| `road` | 一般道路 | 交通監視 | 車両・ナンバー |

---

## 2. プリセット詳細定義

### 2.1 person_priority（人物優先）

**用途**: 人物の詳細特徴を最大限取得

```json
{
  "preset_id": "person_priority",
  "display_name": "人物優先",
  "icon": "user",
  "version": "1.0.0",

  "_meta": {
    "description": "人物の詳細特徴を最大限取得するプリセット。服装、体型、持ち物、髪型など26種類以上の属性を検出。",
    "use_cases": [
      "防犯カメラでの人物特定補助",
      "不審者の特徴記録",
      "スタッフ識別（制服検出）"
    ],
    "detection_tendency": {
      "strength": ["人物の詳細属性", "服装色・パターン", "持ち物検出", "体格推定"],
      "weakness": ["車両検出なし", "動物検出なし", "処理負荷高め"]
    },
    "recommended_locations": ["エントランス", "受付", "廊下", "警戒区域"],
    "false_positive_risk": "低",
    "processing_load": "高"
  },

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": [],
    "conf_threshold": 0.35,
    "max_detections": 10
  },

  "person_features": {
    "enabled": true,
    "detail_level": "maximum",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": true,
      "accessories": true,
      "posture": true,
      "facing": true,
      "hair": true,
      "body_build": true,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": true
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": false
  },

  "suspicious_config": {
    "base_multiplier": 1.0,
    "location_bonus": 0
  },

  "output_schema": "person_detailed"
}
```

### 2.2 balanced（バランス）

**用途**: 汎用監視、人物・車両・動物を均等に検出

```json
{
  "preset_id": "balanced",
  "display_name": "バランス",
  "icon": "scale",
  "version": "1.0.0",

  "_meta": {
    "description": "人物・車両・動物をバランスよく検出する汎用プリセット。初期設定として推奨。",
    "use_cases": [
      "一般的な監視カメラ",
      "複数対象が混在するエリア",
      "プリセット選択に迷った場合のデフォルト"
    ],
    "detection_tendency": {
      "strength": ["幅広い対象を検出", "適度な詳細度", "安定した動作"],
      "weakness": ["特化型に比べ詳細度は控えめ", "ナンバー読取なし"]
    },
    "recommended_locations": ["汎用", "オフィス", "倉庫", "共用部"],
    "false_positive_risk": "中",
    "processing_load": "中"
  },

  "detection": {
    "primary_targets": ["person", "car", "truck", "motorcycle", "dog", "cat"],
    "secondary_targets": ["bicycle", "bus", "bird"],
    "conf_threshold": 0.4,
    "max_detections": 15
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": false
  },

  "scene_analysis": {
    "group_dynamics": false,
    "spatial_occupation": true,
    "activity_estimation": false
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": false
  },

  "suspicious_config": {
    "base_multiplier": 1.0,
    "location_bonus": 0
  },

  "output_schema": "standard"
}
```

### 2.3 high_detection（高検出）

**用途**: 見逃し最小化、低閾値で全検出

```json
{
  "preset_id": "high_detection",
  "display_name": "高検出",
  "icon": "radar",

  "detection": {
    "primary_targets": ["person", "car", "truck", "motorcycle", "dog", "cat", "bird"],
    "secondary_targets": ["bicycle", "bus", "boat"],
    "conf_threshold": 0.25,
    "max_detections": 30
  },

  "person_features": {
    "enabled": true,
    "detail_level": "minimal",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": false,
      "posture": false,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": false
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": false
  },

  "scene_analysis": {
    "group_dynamics": false,
    "spatial_occupation": false,
    "activity_estimation": false
  },

  "frame_diff": {
    "enabled": false
  },

  "suspicious_config": {
    "base_multiplier": 0.8,
    "location_bonus": 0
  },

  "output_schema": "minimal"
}
```

### 2.4 parking（駐車場）

**用途**: 車両管理、ナンバープレート読取

```json
{
  "preset_id": "parking",
  "display_name": "駐車場",
  "icon": "car",
  "version": "1.0.0",

  "_meta": {
    "description": "車両検出とナンバープレート読取に特化。駐車場管理、入退場記録に最適。",
    "use_cases": [
      "駐車場の入退場管理",
      "不正駐車の検出",
      "来客車両の記録",
      "月極契約車両の確認"
    ],
    "detection_tendency": {
      "strength": ["車両タイプ分類", "ナンバー読取（日本式対応）", "車両色検出", "駐車時間追跡"],
      "weakness": ["人物詳細は最小限", "夜間精度低下あり"]
    },
    "recommended_locations": ["駐車場", "車寄せ", "搬入口", "ゲート前"],
    "false_positive_risk": "低",
    "processing_load": "高（OCR含む）"
  },

  "detection": {
    "primary_targets": ["car", "truck", "motorcycle", "bus"],
    "secondary_targets": ["person", "bicycle"],
    "conf_threshold": 0.35,
    "max_detections": 20
  },

  "person_features": {
    "enabled": true,
    "detail_level": "minimal",
    "attributes": {
      "demographics": false,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": false,
      "posture": true,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": false
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": true,
    "license_plate_config": {
      "region": "JP",
      "confidence_threshold": 0.6,
      "enhance_night": true
    }
  },

  "scene_analysis": {
    "group_dynamics": false,
    "spatial_occupation": true,
    "activity_estimation": false,
    "parking_analysis": {
      "enabled": true,
      "detect_entry_exit": true,
      "track_duration": true
    }
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": true,
    "vehicle_movement": true
  },

  "suspicious_config": {
    "base_multiplier": 1.0,
    "location_bonus": 0,
    "night_bonus": 10
  },

  "output_schema": "parking"
}
```

### 2.5 corridor（施設内通路）

**用途**: 通行監視、滞在検知

```json
{
  "preset_id": "corridor",
  "display_name": "施設内通路",
  "icon": "arrow-right",

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": [],
    "conf_threshold": 0.4,
    "max_detections": 10
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": true,
      "hair": false,
      "body_build": false,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": true,
    "traffic_flow": {
      "enabled": true,
      "direction_tracking": true
    }
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": true,
    "loitering_config": {
      "warning_seconds": 60,
      "alert_seconds": 180
    }
  },

  "suspicious_config": {
    "base_multiplier": 1.0,
    "location_bonus": 0,
    "loitering_bonus": 15
  },

  "output_schema": "corridor"
}
```

### 2.6 restricted（警戒区域）

**用途**: サーバールーム、金庫室等の侵入検知

```json
{
  "preset_id": "restricted",
  "display_name": "警戒区域",
  "icon": "shield-alert",
  "version": "1.0.0",

  "_meta": {
    "description": "最高警戒レベル。人物検知で即時アラート。サーバールーム、金庫室等の立入禁止区域向け。",
    "use_cases": [
      "サーバールーム監視",
      "金庫室・重要書類保管庫",
      "立入禁止区域",
      "夜間無人エリア"
    ],
    "detection_tendency": {
      "strength": ["低閾値で見逃し最小化", "全人物属性を記録", "即時アラート", "短時間滞在で警報"],
      "weakness": ["誤検知リスクあり", "車両・動物は対象外", "処理負荷最大"]
    },
    "recommended_locations": ["サーバールーム", "金庫室", "屋上", "非常口"],
    "false_positive_risk": "やや高",
    "processing_load": "高"
  },

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": [],
    "conf_threshold": 0.3,
    "max_detections": 5
  },

  "person_features": {
    "enabled": true,
    "detail_level": "maximum",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": true,
      "accessories": true,
      "posture": true,
      "facing": true,
      "hair": true,
      "body_build": true,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": true
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": true,
    "loitering_config": {
      "warning_seconds": 10,
      "alert_seconds": 30
    },
    "any_detection_alert": true
  },

  "suspicious_config": {
    "base_multiplier": 2.0,
    "location_bonus": 30,
    "immediate_alert_on_detection": true
  },

  "output_schema": "person_detailed"
}
```

### 2.7 entrance（玄関・フロント）

**用途**: 来客対応、受付エリア

```json
{
  "preset_id": "entrance",
  "display_name": "玄関・フロント",
  "icon": "door-open",

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": ["car"],
    "conf_threshold": 0.4,
    "max_detections": 15
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": true,
      "hair": false,
      "body_build": false,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": false,
    "entry_exit_tracking": {
      "enabled": true,
      "direction": "bidirectional"
    }
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": true,
    "loitering_config": {
      "warning_seconds": 120,
      "alert_seconds": 300
    }
  },

  "suspicious_config": {
    "base_multiplier": 0.8,
    "location_bonus": -5,
    "busy_hours_reduction": 10
  },

  "output_schema": "standard"
}
```

### 2.8 dining（飲食ホール）

**用途**: 客席監視、混雑状況把握

```json
{
  "preset_id": "dining",
  "display_name": "飲食ホール",
  "icon": "utensils",

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": [],
    "conf_threshold": 0.45,
    "max_detections": 30
  },

  "person_features": {
    "enabled": true,
    "detail_level": "minimal",
    "attributes": {
      "demographics": false,
      "clothing_color": false,
      "clothing_pattern": false,
      "accessories": false,
      "posture": true,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": false
    }
  },

  "vehicle_features": {
    "enabled": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": false,
    "occupancy_tracking": {
      "enabled": true,
      "seated_detection": true,
      "table_zones": true
    }
  },

  "frame_diff": {
    "enabled": false
  },

  "suspicious_config": {
    "base_multiplier": 0.5,
    "location_bonus": -10
  },

  "output_schema": "occupancy"
}
```

### 2.9 kitchen（厨房）

**用途**: 衛生管理、安全監視

```json
{
  "preset_id": "kitchen",
  "display_name": "厨房",
  "icon": "chef-hat",

  "detection": {
    "primary_targets": ["person"],
    "secondary_targets": [],
    "conf_threshold": 0.4,
    "max_detections": 10
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": false,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": false
    },
    "hygiene_detection": {
      "enabled": true,
      "detect_hat_cap": true,
      "detect_mask": true,
      "detect_apron": true,
      "detect_gloves": true
    }
  },

  "vehicle_features": {
    "enabled": false
  },

  "scene_analysis": {
    "group_dynamics": false,
    "spatial_occupation": true,
    "activity_estimation": true
  },

  "frame_diff": {
    "enabled": false
  },

  "suspicious_config": {
    "base_multiplier": 0.7,
    "location_bonus": 0,
    "no_hygiene_alert": true
  },

  "output_schema": "kitchen"
}
```

### 2.10 campsite（キャンプ場）

**用途**: 野外レジャー施設監視

```json
{
  "preset_id": "campsite",
  "display_name": "キャンプ場",
  "icon": "tent",

  "detection": {
    "primary_targets": ["person", "car", "truck"],
    "secondary_targets": ["dog", "cat", "bear", "bird"],
    "conf_threshold": 0.35,
    "max_detections": 20
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": true
  },

  "animal_features": {
    "enabled": true,
    "wild_animal_alert": true,
    "target_animals": ["bear", "boar", "deer"]
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": false
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": false
  },

  "suspicious_config": {
    "base_multiplier": 1.2,
    "location_bonus": 0,
    "night_bonus": 15,
    "wild_animal_bonus": 30
  },

  "output_schema": "outdoor"
}
```

### 2.11 outdoor（施設屋外）

**用途**: 建物外周、庭園等

```json
{
  "preset_id": "outdoor",
  "display_name": "施設屋外",
  "icon": "tree",

  "detection": {
    "primary_targets": ["person", "car"],
    "secondary_targets": ["dog", "cat", "bird", "motorcycle", "bicycle"],
    "conf_threshold": 0.35,
    "max_detections": 15
  },

  "person_features": {
    "enabled": true,
    "detail_level": "standard",
    "attributes": {
      "demographics": true,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": true,
      "posture": true,
      "facing": true,
      "hair": false,
      "body_build": false,
      "height": true
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": false
  },

  "scene_analysis": {
    "group_dynamics": true,
    "spatial_occupation": true,
    "activity_estimation": true
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": true,
    "loitering_config": {
      "warning_seconds": 180,
      "alert_seconds": 600
    }
  },

  "suspicious_config": {
    "base_multiplier": 1.2,
    "location_bonus": 5,
    "night_bonus": 15
  },

  "output_schema": "outdoor"
}
```

### 2.12 road（一般道路）

**用途**: 交通監視、ナンバー読取

```json
{
  "preset_id": "road",
  "display_name": "一般道路",
  "icon": "road",

  "detection": {
    "primary_targets": ["car", "truck", "bus", "motorcycle"],
    "secondary_targets": ["person", "bicycle"],
    "conf_threshold": 0.35,
    "max_detections": 30
  },

  "person_features": {
    "enabled": true,
    "detail_level": "minimal",
    "attributes": {
      "demographics": false,
      "clothing_color": true,
      "clothing_pattern": false,
      "accessories": false,
      "posture": false,
      "facing": false,
      "hair": false,
      "body_build": false,
      "height": false
    }
  },

  "vehicle_features": {
    "enabled": true,
    "color": true,
    "type": true,
    "license_plate": true,
    "license_plate_config": {
      "region": "JP",
      "confidence_threshold": 0.5,
      "enhance_night": true,
      "track_same_plate": true
    },
    "speed_estimation": {
      "enabled": true,
      "calibration_required": true
    }
  },

  "scene_analysis": {
    "group_dynamics": false,
    "spatial_occupation": false,
    "activity_estimation": false,
    "traffic_flow": {
      "enabled": true,
      "count_vehicles": true,
      "direction_tracking": true
    }
  },

  "frame_diff": {
    "enabled": true,
    "loitering_detection": false,
    "vehicle_movement": true
  },

  "suspicious_config": {
    "base_multiplier": 0.5,
    "location_bonus": 0
  },

  "output_schema": "traffic"
}
```

---

## 3. 出力スキーマ定義

### 3.1 person_detailed

人物詳細特化。`person_priority`, `restricted` で使用。

```json
{
  "persons": [
    {
      "index": 0,
      "bbox": {},
      "confidence": 0.85,
      "demographics": {"gender": "female", "age_group": "adult"},
      "appearance": {
        "height": "tall",
        "build": null,
        "posture": "standing",
        "facing": "front"
      },
      "clothing": {
        "upper": {"color": "white", "pattern": "solid", "sleeve": "long"},
        "lower": {"color": "blue", "type": "pants"},
        "footwear": "boots"
      },
      "accessories": {
        "hat": {"wearing": true, "type": "cap"},
        "glasses": {"wearing": false},
        "mask": {"wearing": false},
        "bag": ["backpack"]
      },
      "hair": {"color": "dark", "length": "medium"}
    }
  ],
  "scene": {
    "person_count": 2,
    "groups": [],
    "activity": ["walking", "standing"]
  },
  "suspicious": {"score": 25, "level": "low"}
}
```

### 3.2 standard

汎用形式。`balanced`, `entrance` で使用。

```json
{
  "detections": {
    "persons": [{"bbox": {}, "color": "white/blue", "accessories": ["backpack"]}],
    "vehicles": [{"bbox": {}, "type": "car", "color": "white"}],
    "animals": []
  },
  "counts": {"person": 2, "vehicle": 1, "animal": 0},
  "suspicious": {"score": 15, "level": "normal"}
}
```

### 3.3 minimal

最小形式。`high_detection` で使用。

```json
{
  "counts": {"person": 5, "vehicle": 3, "animal": 1},
  "primary_event": "human",
  "severity": 1
}
```

### 3.4 parking

駐車場特化。

```json
{
  "vehicles": [
    {
      "bbox": {},
      "type": "car",
      "color": "white",
      "license_plate": {
        "text": "品川 300 あ 12-34",
        "confidence": 0.85,
        "region": "品川"
      },
      "status": "parked",
      "duration_minutes": 45
    }
  ],
  "persons": [{"bbox": {}, "near_vehicle": true}],
  "occupancy": {"total_spots": 20, "occupied": 12},
  "events": ["vehicle_entered", "person_near_vehicle"]
}
```

### 3.5 corridor

通路特化。滞在検知含む。

```json
{
  "persons": [
    {
      "bbox": {},
      "direction": "north",
      "speed": "walking",
      "loitering": {"detected": false, "duration_sec": 0}
    }
  ],
  "traffic": {
    "count_in": 15,
    "count_out": 12,
    "current_occupancy": 3
  },
  "alerts": []
}
```

### 3.6 kitchen

厨房特化。衛生チェック含む。

```json
{
  "persons": [
    {
      "bbox": {},
      "hygiene": {
        "hat_detected": true,
        "mask_detected": true,
        "apron_detected": true,
        "gloves_detected": false,
        "compliant": false,
        "violations": ["gloves_missing"]
      }
    }
  ],
  "compliance_rate": 0.8,
  "alerts": ["staff_without_gloves"]
}
```

### 3.7 occupancy

混雑状況特化。`dining` で使用。

```json
{
  "occupancy": {
    "total_persons": 25,
    "seated": 20,
    "standing": 5,
    "density": "moderate"
  },
  "zones": {
    "zone_a": {"count": 10, "capacity": 15},
    "zone_b": {"count": 15, "capacity": 20}
  }
}
```

### 3.8 outdoor / traffic

屋外・交通特化。

```json
{
  "detections": {
    "persons": [],
    "vehicles": [{"type": "car", "color": "silver", "plate": "..."}],
    "animals": [{"type": "bird", "count": 3}]
  },
  "traffic": {
    "vehicle_count": 45,
    "avg_speed_kmh": 35,
    "direction": {"north": 25, "south": 20}
  }
}
```

---

## 4. 車両系拡張

### 4.1 ナンバープレートOCR

**実装方針**: PaddleOCR軽量版をRKNN変換

```python
class LicensePlateReader:
    """
    日本のナンバープレート読取

    フォーマット: [地名] [分類番号] [ひらがな] [一連番号]
    例: 品川 300 あ 12-34
    """

    def detect_plate(self, image, vehicle_bbox) -> Optional[Dict]:
        """
        Returns:
            {
                "bbox": {"x1": ..., "y1": ..., "x2": ..., "y2": ...},
                "text": "品川 300 あ 12-34",
                "confidence": 0.85,
                "region": "品川",
                "classification": "300",
                "kana": "あ",
                "number": "12-34"
            }
        """
```

### 4.2 車両タイプ詳細

```python
VEHICLE_TYPES = {
    "car": {
        "subtypes": ["sedan", "suv", "wagon", "compact", "sports"],
        "commercial": False
    },
    "truck": {
        "subtypes": ["light", "medium", "heavy", "trailer"],
        "commercial": True
    },
    "bus": {
        "subtypes": ["city", "coach", "minibus"],
        "commercial": True
    },
    "motorcycle": {
        "subtypes": ["standard", "scooter", "large"],
        "commercial": False
    }
}
```

---

## 5. CameraDetailModal UI設計

### 5.1 プリセット選択セクション

```
┌─ カメラプリセット ─────────────────────────────┐
│                                               │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │ 👤      │ │ ⚖️      │ │ 📡      │         │
│  │人物優先 │ │バランス │ │高検出   │         │
│  └─────────┘ └─────────┘ └─────────┘         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │ 🚗      │ │ 🚶      │ │ 🛡️      │         │
│  │駐車場   │ │通路     │ │警戒区域 │         │
│  └─────────┘ └─────────┘ └─────────┘         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │ 🚪      │ │ 🍽️      │ │ 👨‍🍳      │         │
│  │玄関     │ │飲食     │ │厨房     │         │
│  └─────────┘ └─────────┘ └─────────┘         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐         │
│  │ ⛺      │ │ 🌳      │ │ 🛣️      │         │
│  │キャンプ │ │屋外     │ │道路     │         │
│  └─────────┘ └─────────┘ └─────────┘         │
│                                               │
│  選択中: [バランス] ✓                         │
│                                               │
│  ┌─ カスタマイズ ──────────────────────────┐ │
│  │ [ ] プリセットを上書きしてカスタマイズ   │ │
│  └─────────────────────────────────────────┘ │
└───────────────────────────────────────────────┘
```

### 5.2 cameras テーブル拡張

```sql
ALTER TABLE cameras
ADD COLUMN preset_id VARCHAR(32) DEFAULT 'balanced' COMMENT 'プリセットID',
ADD COLUMN preset_overrides JSON NULL COMMENT 'プリセット上書き設定';

CREATE INDEX idx_cameras_preset ON cameras(preset_id);
```

---

## 6. is21リクエスト/レスポンス変更

### 6.1 リクエスト追加パラメータ

```python
# hints_json 拡張
{
  "preset_id": "parking",
  "output_schema": "parking",

  # 既存
  "location_type": "parking",
  "expected_objects": ["car", "truck", "person"],

  # 新規: 前回フレーム情報
  "previous_frame": {
    "captured_at": "...",
    "vehicle_count": 5,
    "person_count": 1
  }
}
```

### 6.2 レスポンス構造

```python
{
  "schema_version": "2026-01-02.1",
  "preset_id": "parking",
  "output_schema": "parking",

  # スキーマに応じた構造化出力
  "result": {
    # parking スキーマの場合
    "vehicles": [...],
    "persons": [...],
    "occupancy": {...}
  },

  # 共通フィールド
  "primary_event": "vehicle",
  "severity": 1,
  "suspicious": {...},
  "processing_ms": {...},
  "camera_status": {...}
}
```

---

## 7. 共有プリセット定義ファイル

### 7.1 設計方針

**重要**: プリセット定義はis22とis21の両方が静的設定として保持する。

```
is22/
├── config/
│   └── presets/
│       └── camera_presets_v1.0.0.json  ← 共有定義ファイル

is21/
├── config/
│   └── presets/
│       └── camera_presets_v1.0.0.json  ← 同一ファイル
```

### 7.2 プリセット定義ファイル構造

```json
{
  "$schema": "https://aranea.mijeos.com/schemas/camera-presets-v1.json",
  "version": "1.0.0",
  "updated_at": "2026-01-02T00:00:00Z",

  "presets": {
    "person_priority": {
      "preset_id": "person_priority",
      "display_name": "人物優先",
      "icon": "user",
      "version": "1.0.0",

      "_meta": {
        "description": "人物の詳細特徴を最大限取得するプリセット。...",
        "use_cases": ["防犯カメラでの人物特定補助", "..."],
        "detection_tendency": {
          "strength": ["人物の詳細属性", "..."],
          "weakness": ["車両検出なし", "..."]
        },
        "recommended_locations": ["エントランス", "受付", "..."],
        "false_positive_risk": "低",
        "processing_load": "高"
      },

      "detection": { ... },
      "person_features": { ... },
      "vehicle_features": { ... },
      "scene_analysis": { ... },
      "frame_diff": { ... },
      "suspicious_config": { ... },
      "output_schema": "person_detailed"
    },
    "balanced": { ... },
    "parking": { ... }
    // ... 他のプリセット
  },

  "output_schemas": {
    "person_detailed": { ... },
    "standard": { ... },
    "parking": { ... }
    // ... 他のスキーマ
  }
}
```

### 7.3 バージョン管理

| 項目 | 説明 |
|------|------|
| ファイル名 | `camera_presets_v{MAJOR}.{MINOR}.{PATCH}.json` |
| 互換性 | MAJOR変更時は後方互換なし、MINOR/PATCHは後方互換あり |
| 配布 | is22/is21デプロイ時に同梱、または起動時にリモート取得 |
| 更新 | チューニング後に新バージョンを両システムに配布 |

### 7.4 チューニング履歴記録

```json
{
  "_tuning_history": [
    {
      "date": "2026-01-02",
      "version": "1.0.0",
      "changes": "初版リリース",
      "author": "system"
    },
    {
      "date": "2026-01-10",
      "version": "1.0.1",
      "changes": "restricted: conf_threshold 0.3→0.25（見逃し削減）",
      "author": "operator",
      "evidence": "incident_report_20260108.md"
    }
  ]
}
```

---

## 8. データ保存仕様（preset_idトレーサビリティ）

### 8.1 原則

**すべてのデータにpreset_idを含める**:
- is21リクエスト
- is21レスポンス
- ローカルDB（detection_logs）
- BigQuery同期
- LLM/チャット送信

### 8.2 is21リクエスト

```json
{
  "camera_id": "cam-xxx",
  "schema_version": "2026-01-02.1",
  "captured_at": "2026-01-02T10:30:00Z",
  "preset_id": "parking",           // ← 必須
  "preset_version": "1.0.0",        // ← プリセットバージョン
  "output_schema": "parking",       // ← 期待する出力形式
  "hints_json": {
    "location_type": "parking",
    "expected_objects": ["car", "truck"]
  }
}
```

### 8.3 is21レスポンス

```json
{
  "schema_version": "2026-01-02.1",
  "camera_id": "cam-xxx",
  "captured_at": "2026-01-02T10:30:00Z",

  "preset_id": "parking",           // ← リクエストをエコーバック
  "preset_version": "1.0.0",        // ← 使用したプリセットバージョン
  "output_schema": "parking",       // ← 実際に使用した出力形式

  "primary_event": "vehicle",
  "severity": 1,
  "detected": true,

  "result": {
    // output_schemaに応じた構造
  },

  "processing_ms": { ... }
}
```

### 8.4 ローカルDB（detection_logs）

```sql
-- migration追加
ALTER TABLE detection_logs
ADD COLUMN preset_id VARCHAR(32) NOT NULL DEFAULT 'balanced' COMMENT '使用プリセットID',
ADD COLUMN preset_version VARCHAR(16) NULL COMMENT 'プリセットバージョン',
ADD COLUMN output_schema VARCHAR(32) NULL COMMENT '出力スキーマ名';

CREATE INDEX idx_logs_preset ON detection_logs(preset_id, captured_at);
```

### 8.5 BigQuery同期

| カラム | 型 | 説明 |
|--------|-----|------|
| preset_id | STRING | 使用プリセットID |
| preset_version | STRING | プリセットバージョン |
| output_schema | STRING | 出力スキーマ名 |

### 8.6 LLM/チャット送信

```json
{
  "query": "直近1時間の駐車場の異常を教えて",
  "context": {
    "logs": [
      {
        "log_id": 12345,
        "preset_id": "parking",      // ← 含める
        "preset_version": "1.0.0",
        "output_schema": "parking",
        "result": { ... }
      }
    ]
  }
}
```

---

## 9. CameraDetailModal UI設計（プリセット説明表示）

### 9.1 プリセット選択ドロップダウン

```
┌─ プリセット選択 ──────────────────────────────────────┐
│                                                        │
│  [バランス ▼]                                         │
│                                                        │
│  ┌─ プリセット説明 ────────────────────────────────┐  │
│  │                                                    │  │
│  │  📋 バランス (v1.0.0)                             │  │
│  │                                                    │  │
│  │  人物・車両・動物をバランスよく検出する汎用       │  │
│  │  プリセット。初期設定として推奨。                 │  │
│  │                                                    │  │
│  │  ───────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  ✅ 強み                                          │  │
│  │  • 幅広い対象を検出                               │  │
│  │  • 適度な詳細度                                   │  │
│  │  • 安定した動作                                   │  │
│  │                                                    │  │
│  │  ⚠️ 弱み                                          │  │
│  │  • 特化型に比べ詳細度は控えめ                     │  │
│  │  • ナンバー読取なし                               │  │
│  │                                                    │  │
│  │  ───────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  📍 推奨設置場所                                  │  │
│  │  汎用, オフィス, 倉庫, 共用部                     │  │
│  │                                                    │  │
│  │  🎯 用途例                                        │  │
│  │  • 一般的な監視カメラ                             │  │
│  │  • 複数対象が混在するエリア                       │  │
│  │  • プリセット選択に迷った場合のデフォルト         │  │
│  │                                                    │  │
│  │  ───────────────────────────────────────────────  │  │
│  │                                                    │  │
│  │  ⚡ 処理負荷: 中    🎯 誤検知リスク: 中          │  │
│  │                                                    │  │
│  └────────────────────────────────────────────────────┘  │
│                                                        │
└────────────────────────────────────────────────────────┘
```

### 9.2 プリセットカード一覧（グリッド表示オプション）

```
┌─ プリセット選択 ──────────────────────────────────────┐
│  [📋 リスト] [▣ グリッド]                            │
│                                                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│  │ 👤        │ │ ⚖️  ✓    │ │ 📡        │              │
│  │ 人物優先  │ │ バランス │ │ 高検出    │              │
│  │ 処理:高   │ │ 処理:中  │ │ 処理:低   │              │
│  └──────────┘ └──────────┘ └──────────┘              │
│                                                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐              │
│  │ 🚗        │ │ 🚶        │ │ 🛡️        │              │
│  │ 駐車場    │ │ 通路      │ │ 警戒区域  │              │
│  │ 処理:高   │ │ 処理:中   │ │ 処理:高   │              │
│  └──────────┘ └──────────┘ └──────────┘              │
│                                                        │
│  ... (その他)                                          │
│                                                        │
│  ─────────────────────────────────────────────────     │
│  選択中のプリセット詳細が下に表示                      │
└────────────────────────────────────────────────────────┘
```

### 9.3 TypeScript型定義

```typescript
interface PresetMeta {
  description: string;
  use_cases: string[];
  detection_tendency: {
    strength: string[];
    weakness: string[];
  };
  recommended_locations: string[];
  false_positive_risk: 'low' | 'medium' | 'high';
  processing_load: 'low' | 'medium' | 'high';
}

interface CameraPreset {
  preset_id: string;
  display_name: string;
  icon: string;
  version: string;
  _meta: PresetMeta;
  detection: DetectionConfig;
  person_features: PersonFeaturesConfig;
  vehicle_features: VehicleFeaturesConfig;
  scene_analysis: SceneAnalysisConfig;
  frame_diff: FrameDiffConfig;
  suspicious_config: SuspiciousConfig;
  output_schema: string;
}

// プリセット説明表示コンポーネント
const PresetInfoPanel: React.FC<{ preset: CameraPreset }> = ({ preset }) => {
  const { _meta } = preset;
  return (
    <div className="preset-info-panel">
      <h3>{preset.display_name} (v{preset.version})</h3>
      <p>{_meta.description}</p>

      <div className="strength">
        <h4>✅ 強み</h4>
        <ul>{_meta.detection_tendency.strength.map(s => <li key={s}>{s}</li>)}</ul>
      </div>

      <div className="weakness">
        <h4>⚠️ 弱み</h4>
        <ul>{_meta.detection_tendency.weakness.map(w => <li key={w}>{w}</li>)}</ul>
      </div>

      <div className="locations">
        <h4>📍 推奨設置場所</h4>
        <p>{_meta.recommended_locations.join(', ')}</p>
      </div>

      <div className="use-cases">
        <h4>🎯 用途例</h4>
        <ul>{_meta.use_cases.map(u => <li key={u}>{u}</li>)}</ul>
      </div>

      <div className="metrics">
        <span>⚡ 処理負荷: {_meta.processing_load}</span>
        <span>🎯 誤検知リスク: {_meta.false_positive_risk}</span>
      </div>
    </div>
  );
};
```

---

## 10. EventLogPane表示テンプレート

### 10.1 設計方針

- **LLM不使用**: 固定テンプレート + 変数埋め込み
- **プリセット別**: 各プリセットに最適化した表示形式
- **一目で理解**: 検知内容が即座にわかる簡潔な表現

### 10.2 プリセット別テンプレート

#### person_priority（人物優先）
```
[10:30:15] 📍カメラ名
👤 成人女性 | 白上着・青ズボン | バックパック
   身長:高め | 立位 | 正面向き
```

#### balanced（バランス）
```
[10:30:15] 📍カメラ名
👤×2 🚗×1 | 人物: 男性・女性 | 車両: 白セダン
```

#### high_detection（高検出）
```
[10:30:15] 📍カメラ名
検出: 👤5 🚗3 🐕1 | conf: 0.45
```

#### parking（駐車場）
```
[10:30:15] 📍カメラ名
🚗 白セダン [品川 300 あ 12-34] 入場
   駐車時間: 45分 | 空き: 8/20台
```

```
[10:30:15] 📍カメラ名
⚠️ 🚗 不明車両 [読取不可] 駐車場内徘徊
```

#### corridor（通路）
```
[10:30:15] 📍カメラ名
🚶 通過: 2名→北 | 現在滞在: 3名
```

```
[10:30:15] 📍カメラ名
⚠️ 滞在警告: 1名が2分以上停止中
```

#### restricted（警戒区域）
```
[10:30:15] 📍カメラ名
🚨 人物検知！男性・黒上着・しゃがみ姿勢
   滞在: 15秒 | suspicious: 85/100
```

#### entrance（玄関・フロント）
```
[10:30:15] 📍カメラ名
🚪 来客: 成人男性 | スーツ・手提げ鞄
```

#### dining（飲食ホール）
```
[10:30:15] 📍カメラ名
🍽️ 混雑: 中 | 着席25名 / 立位5名
   ZoneA: 10/15 | ZoneB: 15/20
```

#### kitchen（厨房）
```
[10:30:15] 📍カメラ名
👨‍🍳 スタッフ3名 | 衛生チェック
   ✅帽子 ✅マスク ✅エプロン ❌手袋(1名)
```

```
[10:30:15] 📍カメラ名
⚠️ 衛生違反: 手袋未着用スタッフあり
```

#### campsite（キャンプ場）
```
[10:30:15] 📍カメラ名
⛺ 👤3 🚗2 | 車両: [札幌 500 す 78-90]
```

```
[10:30:15] 📍カメラ名
🐻 野生動物検知！クマの可能性
```

#### outdoor（施設屋外）
```
[10:30:15] 📍カメラ名
🌳 👤2名 通過 | 🚗1台 駐車中
```

```
[10:30:15] 📍カメラ名
⚠️ 外周滞在: 1名が5分以上 | 夜間
```

#### road（一般道路）
```
[10:30:15] 📍カメラ名
🛣️ 通過: 🚗12 🚚3 🏍️2 | 平均35km/h
   北行: 10台 | 南行: 7台
```

### 10.3 共通要素

#### 時刻表示
```
[HH:MM:SS]  ← 時分秒
```

#### 重要度バッジ
```
severity=0: (表示なし)
severity=1: 🔵
severity=2: 🟡
severity=3: 🔴
```

#### suspiciousレベル
```
normal (0-29):   (表示なし)
low (30-49):     ⚡
medium (50-69):  ⚠️
high (70-89):    🚨
critical (90+):  🚨🚨
```

### 10.4 テンプレート定義構造

```typescript
interface DisplayTemplate {
  preset_id: string;

  // メインテンプレート（通常検知）
  normal: {
    format: string;        // テンプレート文字列
    variables: string[];   // 使用する変数リスト
  };

  // 警告テンプレート（異常検知）
  warning?: {
    condition: string;     // 発動条件
    format: string;
    variables: string[];
  };

  // アイコンマッピング
  icons: {
    person: string;
    vehicle: string;
    animal: string;
    alert: string;
  };
}

// 例: parking
const parkingTemplate: DisplayTemplate = {
  preset_id: "parking",
  normal: {
    format: "🚗 {vehicle_color}{vehicle_type} [{plate}] {action}\n   駐車時間: {duration} | 空き: {available}/{total}台",
    variables: ["vehicle_color", "vehicle_type", "plate", "action", "duration", "available", "total"]
  },
  warning: {
    condition: "plate == null || loitering == true",
    format: "⚠️ 🚗 不明車両 [{plate_status}] {warning_message}",
    variables: ["plate_status", "warning_message"]
  },
  icons: {
    person: "👤",
    vehicle: "🚗",
    animal: "🐕",
    alert: "⚠️"
  }
};
```

### 10.5 テンプレートレンダリング

```typescript
function renderEventLog(
  preset: CameraPreset,
  template: DisplayTemplate,
  result: AnalyzeResult,
  camera: Camera
): string {
  const timestamp = formatTime(result.captured_at);
  const header = `[${timestamp}] 📍${camera.display_name}`;

  // 警告条件チェック
  const useWarning = template.warning &&
    evaluateCondition(template.warning.condition, result);

  const tpl = useWarning ? template.warning : template.normal;

  // 変数展開
  let body = tpl.format;
  for (const varName of tpl.variables) {
    const value = extractVariable(varName, result, preset);
    body = body.replace(`{${varName}}`, value);
  }

  // severity/suspiciousバッジ追加
  const badge = getSeverityBadge(result.severity, result.suspicious?.level);

  return `${header}\n${badge}${body}`;
}

// 変数抽出ヘルパー
function extractVariable(name: string, result: AnalyzeResult, preset: CameraPreset): string {
  switch (name) {
    case "vehicle_color":
      return result.result?.vehicles?.[0]?.color || "不明";
    case "vehicle_type":
      return translateVehicleType(result.result?.vehicles?.[0]?.type);
    case "plate":
      return result.result?.vehicles?.[0]?.license_plate?.text || "読取不可";
    case "person_count":
      return String(result.count_hint || 0);
    case "person_gender":
      return translateGender(result.person_details?.[0]?.meta?.gender);
    case "clothing":
      return formatClothing(result.person_details?.[0]);
    // ... その他の変数
  }
}

// 日本語変換
const TRANSLATIONS = {
  vehicle_type: {
    car: "乗用車", sedan: "セダン", suv: "SUV",
    truck: "トラック", bus: "バス", motorcycle: "バイク"
  },
  gender: {
    male: "男性", female: "女性", unknown: "不明"
  },
  color: {
    white: "白", black: "黒", silver: "銀", red: "赤",
    blue: "青", green: "緑", yellow: "黄"
  }
};
```

### 10.6 EventLogPane実装イメージ

```
┌─ AI検知ログ ─────────────────────────────────────┐
│ [フィルタ: 全て▼] [プリセット: 全て▼] [時間: 1h▼] │
├──────────────────────────────────────────────────┤
│                                                    │
│ [10:32:45] 📍駐車場A                              │
│ 🔵🚗 白セダン [品川 300 あ 12-34] 入場            │
│    駐車時間: 0分 | 空き: 7/20台                   │
│                                                    │
│ [10:31:20] 📍サーバールーム                       │
│ 🔴🚨 人物検知！男性・作業服・立位                 │
│    滞在: 45秒 | suspicious: 92/100                │
│                                                    │
│ [10:30:15] 📍厨房                                 │
│ 🟡⚠️ 衛生違反: 手袋未着用スタッフあり             │
│    👨‍🍳 スタッフ3名 ✅帽子 ✅マスク ❌手袋        │
│                                                    │
│ [10:29:00] 📍正面玄関                             │
│ 🔵🚪 来客: 成人女性 | スーツ・手提げ鞄           │
│                                                    │
│ [10:28:30] 📍通路B                                │
│ 🚶 通過: 3名→北 | 現在滞在: 1名                   │
│                                                    │
│ ─────────────────────────────────────────────    │
│ [さらに読み込む]                                   │
└──────────────────────────────────────────────────┘
```

---

## 11. 実装タスク

### Phase 1: 基盤
- [ ] 共有プリセット定義JSON作成（camera_presets_v1.0.0.json）
- [ ] is22: プリセット読み込み・キャッシュ機構
- [ ] is21: プリセット読み込み・キャッシュ機構
- [ ] cameras.preset_id, preset_version カラム追加
- [ ] detection_logs.preset_id, preset_version, output_schema カラム追加

### Phase 2: UI
- [ ] CameraDetailModal プリセット選択ドロップダウン
- [ ] PresetInfoPanel コンポーネント実装
- [ ] プリセット変更時の確認ダイアログ
- [ ] EventLogPane表示テンプレート実装
- [ ] プリセット別テンプレートJSON定義
- [ ] 日本語翻訳テーブル（車両タイプ、色、性別等）

### Phase 3: is21対応
- [ ] preset_id/preset_version リクエスト受け取り
- [ ] output_schema別レスポンス生成
- [ ] レスポンスにpreset_id/preset_version/output_schemaエコーバック

### Phase 4: データ連携
- [ ] PollingOrchestrator: リクエストにpreset情報含める
- [ ] DetectionLogService: preset情報をDB保存
- [ ] BQ同期: preset情報含める
- [ ] LLM送信: preset情報含める

### Phase 5: チューニング基盤
- [ ] プリセットバージョン履歴管理
- [ ] プリセット別検知精度ダッシュボード
- [ ] A/Bテスト機能（同一カメラで異なるプリセット比較）
