# Paraclate スキーマ定義書

## 概要

本ドキュメントはaraneaSDK SCHEMA_SPEC.mdに準拠し、Paraclateシステムで使用するスキーマを定義する。

### スキーマの棲み分け

| スキーマ | 用途 | 管理責任 |
|---------|------|---------|
| userObject | デバイス基本情報（共通） | araneaSDK（既存使用） |
| userObject_detail | デバイス詳細情報（Type別） | IS21/22開発ライン |
| typeSettings | Type定義・スキーマ正本 | IS21/22開発ライン |
| araneaDeviceStates | 状態キャッシュ | 自動生成 |

---

## 1. is22 CamServer スキーマ

### 1.1 基本情報

| 項目 | 値 |
|------|------|
| TypeDomain | araneaDevice |
| Type | aranea_ar-is22 |
| Prefix | 3 |
| ProductType | 022 |
| ProductCode | 0000 |
| LacisID形式 | `3022{MAC12桁}0000` |

### 1.2 typeSettings/araneaDevice/aranea_ar-is22

```json
{
  "displayName": "Paraclate CamServer",
  "description": "RTSPカメラ総合管理サーバー。最大30台のカメラを監視・インデックス化・AI推論・通知を行う",
  "version": 1,
  "productType": "022",
  "productCodes": ["0000"],

  "stateSchema": {
    "type": "object",
    "properties": {
      "serverStatus": {
        "type": "string",
        "enum": ["running", "stopped", "error", "maintenance"],
        "description": "サーバー稼働状態"
      },
      "cameraStats": {
        "type": "object",
        "properties": {
          "total": { "type": "integer", "description": "登録カメラ総数" },
          "online": { "type": "integer", "description": "オンラインカメラ数" },
          "offline": { "type": "integer", "description": "オフラインカメラ数" },
          "error": { "type": "integer", "description": "エラー状態カメラ数" }
        },
        "required": ["total", "online", "offline"]
      },
      "inference": {
        "type": "object",
        "properties": {
          "status": {
            "type": "string",
            "enum": ["idle", "processing", "error", "disabled"],
            "description": "推論エンジン状態"
          },
          "queueSize": { "type": "integer", "description": "推論待ちキューサイズ" },
          "lastProcessedAt": { "type": "string", "format": "date-time" }
        }
      },
      "storage": {
        "type": "object",
        "properties": {
          "usagePercent": { "type": "number", "minimum": 0, "maximum": 100 },
          "snapshotCount": { "type": "integer" },
          "oldestSnapshotAt": { "type": "string", "format": "date-time" }
        }
      },
      "pollingCycle": {
        "type": "object",
        "properties": {
          "currentCycle": { "type": "integer" },
          "lastCycleAt": { "type": "string", "format": "date-time" },
          "avgCycleDurationMs": { "type": "integer" }
        }
      }
    },
    "required": ["serverStatus", "cameraStats"]
  },

  "configSchema": {
    "type": "object",
    "properties": {
      "polling": {
        "type": "object",
        "properties": {
          "intervalSec": { "type": "integer", "minimum": 5, "maximum": 300, "default": 60 },
          "timeoutMainSec": { "type": "integer", "minimum": 5, "maximum": 30, "default": 10 },
          "timeoutSubSec": { "type": "integer", "minimum": 10, "maximum": 60, "default": 20 }
        }
      },
      "inference": {
        "type": "object",
        "properties": {
          "enabled": { "type": "boolean", "default": true },
          "mode": {
            "type": "string",
            "enum": ["all", "differential", "motion_only"],
            "default": "differential"
          },
          "maxConcurrent": { "type": "integer", "minimum": 1, "maximum": 10, "default": 4 }
        }
      },
      "paraclate": {
        "type": "object",
        "properties": {
          "endpoint": { "type": "string", "format": "uri" },
          "summaryIntervalMinutes": { "type": "integer", "minimum": 15, "maximum": 1440, "default": 60 },
          "grandSummaryTimes": {
            "type": "array",
            "items": { "type": "string", "pattern": "^[0-2][0-9]:[0-5][0-9]$" },
            "default": ["09:00", "17:00", "21:00"]
          },
          "reportDetailLevel": {
            "type": "string",
            "enum": ["concise", "standard", "detailed"],
            "default": "standard"
          }
        }
      },
      "storage": {
        "type": "object",
        "properties": {
          "snapshotRetentionHours": { "type": "integer", "minimum": 1, "maximum": 168, "default": 24 },
          "maxSnapshotsPerCamera": { "type": "integer", "minimum": 10, "maximum": 1000, "default": 100 }
        }
      }
    }
  },

  "capabilities": [
    "camera_management",
    "rtsp_capture",
    "ai_inference",
    "event_detection",
    "paraclate_reporting",
    "go2rtc_streaming",
    "web_dashboard"
  ],

  "semanticTags": [
    "カメラ監視",
    "AI推論",
    "イベント検出",
    "サマリーレポート",
    "RTSPストリーミング"
  ]
}
```

### 1.3 userObject_detail/is22CamServer

```json
{
  "firmware": {
    "version": "0.1.0",
    "buildDate": "2026-01-10T00:00:00Z",
    "modules": ["axum", "sqlx", "ffmpeg", "go2rtc"]
  },

  "config": {
    "polling": {
      "intervalSec": 60,
      "timeoutMainSec": 10,
      "timeoutSubSec": 20
    },
    "inference": {
      "enabled": true,
      "mode": "differential",
      "maxConcurrent": 4
    },
    "paraclate": {
      "endpoint": "https://us-central1-mobesorder.cloudfunctions.net/paraclateAPI",
      "summaryIntervalMinutes": 60,
      "grandSummaryTimes": ["09:00", "17:00", "21:00"],
      "reportDetailLevel": "standard"
    },
    "storage": {
      "snapshotRetentionHours": 24,
      "maxSnapshotsPerCamera": 100
    }
  },

  "status": {
    "online": true,
    "lastSeen": "2026-01-10T00:00:00Z",
    "heap": 174000000,
    "uptime": 86400
  },

  "network": {
    "ip": "192.168.125.246",
    "gateway": "192.168.125.1",
    "subnet": "255.255.255.0"
  },

  "cameras": {
    "managedCameras": [
      "3801AABBCCDDEEFF0001",
      "3801112233445566HIKVISION"
    ],
    "subnets": [
      {
        "fid": "0150",
        "cidr": "192.168.125.0/24",
        "tid": "T2025120621041161827"
      },
      {
        "fid": "0151",
        "cidr": "192.168.126.0/24",
        "tid": "T2025120621041161827"
      }
    ]
  },

  "is21Connection": {
    "endpoint": "http://192.168.3.240:9000",
    "activated": true,
    "lastHealthCheckAt": "2026-01-10T00:00:00Z"
  }
}
```

---

## 2. is21 Inference Server スキーマ

### 2.1 基本情報

| 項目 | 値 |
|------|------|
| TypeDomain | araneaDevice |
| Type | aranea_ar-is21 |
| Prefix | 3 |
| ProductType | 021 |
| ProductCode | 0001 |
| LacisID形式 | `3021{MAC12桁}0001` |

### 2.2 typeSettings/araneaDevice/aranea_ar-is21

```json
{
  "displayName": "Paraclate Inference Server",
  "description": "AI推論サーバー。カメラスナップショットの物体検出・シーン分析・異常検知を行う",
  "version": 1,
  "productType": "021",
  "productCodes": ["0001"],

  "stateSchema": {
    "type": "object",
    "properties": {
      "serverStatus": {
        "type": "string",
        "enum": ["running", "stopped", "error", "maintenance"],
        "description": "サーバー稼働状態"
      },
      "inferenceEngine": {
        "type": "object",
        "properties": {
          "status": {
            "type": "string",
            "enum": ["ready", "busy", "error", "initializing"],
            "description": "推論エンジン状態"
          },
          "modelLoaded": { "type": "boolean", "description": "モデルロード状態" },
          "gpuAvailable": { "type": "boolean", "description": "GPU利用可能" },
          "gpuMemoryUsedMb": { "type": "integer", "description": "GPU使用メモリ(MB)" }
        },
        "required": ["status", "modelLoaded"]
      },
      "statistics": {
        "type": "object",
        "properties": {
          "totalInferences": { "type": "integer", "description": "総推論回数" },
          "successRate": { "type": "number", "minimum": 0, "maximum": 100, "description": "成功率(%)" },
          "avgLatencyMs": { "type": "integer", "description": "平均推論時間(ms)" },
          "queueSize": { "type": "integer", "description": "待機キューサイズ" }
        }
      },
      "connectedClients": {
        "type": "object",
        "properties": {
          "is22Count": { "type": "integer", "description": "接続中のIS22数" },
          "lastRequestAt": { "type": "string", "format": "date-time" }
        }
      }
    },
    "required": ["serverStatus", "inferenceEngine"]
  },

  "configSchema": {
    "type": "object",
    "properties": {
      "inference": {
        "type": "object",
        "properties": {
          "maxConcurrent": { "type": "integer", "minimum": 1, "maximum": 8, "default": 2 },
          "timeoutSec": { "type": "integer", "minimum": 5, "maximum": 120, "default": 30 },
          "batchSize": { "type": "integer", "minimum": 1, "maximum": 16, "default": 1 },
          "priorityMode": {
            "type": "string",
            "enum": ["fifo", "round_robin", "priority"],
            "default": "fifo"
          }
        }
      },
      "model": {
        "type": "object",
        "properties": {
          "detectionThreshold": { "type": "number", "minimum": 0.1, "maximum": 1.0, "default": 0.5 },
          "nmsThreshold": { "type": "number", "minimum": 0.1, "maximum": 1.0, "default": 0.4 },
          "maxDetections": { "type": "integer", "minimum": 1, "maximum": 100, "default": 20 }
        }
      },
      "paraclate": {
        "type": "object",
        "properties": {
          "endpoint": { "type": "string", "format": "uri" },
          "reportIntervalMinutes": { "type": "integer", "minimum": 5, "maximum": 60, "default": 15 }
        }
      }
    }
  },

  "capabilities": [
    "object_detection",
    "scene_analysis",
    "anomaly_detection",
    "batch_inference",
    "gpu_acceleration",
    "paraclate_reporting"
  ],

  "semanticTags": [
    "AI推論",
    "物体検出",
    "シーン分析",
    "異常検知",
    "画像処理"
  ]
}
```

### 2.3 userObject_detail/is21InferenceServer

```json
{
  "firmware": {
    "version": "0.1.0",
    "buildDate": "2026-01-10T00:00:00Z",
    "modules": ["onnxruntime", "opencv", "cuda"]
  },

  "config": {
    "inference": {
      "maxConcurrent": 2,
      "timeoutSec": 30,
      "batchSize": 1,
      "priorityMode": "fifo"
    },
    "model": {
      "detectionThreshold": 0.5,
      "nmsThreshold": 0.4,
      "maxDetections": 20
    },
    "paraclate": {
      "endpoint": "https://us-central1-mobesorder.cloudfunctions.net/paraclateAPI",
      "reportIntervalMinutes": 15
    }
  },

  "status": {
    "online": true,
    "lastSeen": "2026-01-10T00:00:00Z",
    "heap": 512000000,
    "uptime": 86400
  },

  "network": {
    "ip": "192.168.3.240",
    "gateway": "192.168.3.1",
    "subnet": "255.255.255.0"
  },

  "hardware": {
    "gpu": "NVIDIA RTX 3060",
    "gpuMemoryMb": 12288,
    "cudaVersion": "12.0",
    "cpuCores": 8,
    "ramGb": 32
  },

  "connectedServers": [
    {
      "lacisId": "3022E051D815448B0001",
      "ip": "192.168.125.246",
      "name": "HALE京都丹波口"
    }
  ]
}
```

---

## 3. is801 ParaclateCamera スキーマ

### 3.1 基本情報

| 項目 | 値 |
|------|------|
| TypeDomain | araneaDevice |
| Type | aranea_ar-is801 |
| Prefix | 3 |
| ProductType | 801 |
| ProductCode | 0000（固定） |
| LacisID形式 | `3801{MAC12桁}0000` |

### 3.2 typeSettings/araneaDevice/aranea_ar-is801

```json
{
  "displayName": "Paraclate Camera",
  "description": "is22 CamServerに接続されるRTSPカメラの仮想araneaDevice表現",
  "version": 1,
  "productType": "801",
  "productCodes": ["0000"],

  "stateSchema": {
    "type": "object",
    "properties": {
      "connectionStatus": {
        "type": "string",
        "enum": ["online", "offline", "error", "unknown"],
        "description": "接続状態"
      },
      "lastCapturedAt": {
        "type": "string",
        "format": "date-time",
        "description": "最終キャプチャ時刻"
      },
      "lastDetection": {
        "type": "object",
        "properties": {
          "timestamp": { "type": "string", "format": "date-time" },
          "tags": {
            "type": "array",
            "items": { "type": "string" },
            "description": "検出タグ (person, vehicle, animal等)"
          },
          "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
        }
      },
      "streamHealth": {
        "type": "object",
        "properties": {
          "fps": { "type": "number" },
          "bitrate": { "type": "integer" },
          "resolution": { "type": "string", "pattern": "^[0-9]+x[0-9]+$" },
          "codec": { "type": "string" }
        }
      },
      "errorCount24h": {
        "type": "integer",
        "description": "24時間以内のエラー回数"
      }
    },
    "required": ["connectionStatus"]
  },

  "configSchema": {
    "type": "object",
    "properties": {
      "rtsp": {
        "type": "object",
        "properties": {
          "mainStreamUrl": { "type": "string", "description": "メインストリームURL" },
          "subStreamUrl": { "type": "string", "description": "サブストリームURL" },
          "username": { "type": "string" },
          "password": { "type": "string", "description": "暗号化保存" }
        },
        "required": ["mainStreamUrl"]
      },
      "inference": {
        "type": "object",
        "properties": {
          "enabled": { "type": "boolean", "default": true },
          "preset": {
            "type": "string",
            "enum": ["detection_general", "detection_person", "detection_vehicle", "motion_only", "disabled"],
            "default": "detection_general"
          },
          "confidenceThreshold": { "type": "number", "minimum": 0, "maximum": 1, "default": 0.5 },
          "customThresholds": {
            "type": "object",
            "additionalProperties": { "type": "number", "minimum": 0, "maximum": 1 }
          }
        }
      },
      "context": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "カメラ表示名" },
          "location": { "type": "string", "description": "設置場所説明" },
          "purpose": { "type": "string", "description": "監視目的" },
          "notes": { "type": "string", "description": "運用メモ" }
        }
      },
      "fitMode": {
        "type": "string",
        "enum": ["contain", "cover", "fill"],
        "default": "contain",
        "description": "UI表示時のアスペクト比"
      }
    },
    "required": ["rtsp"]
  },

  "capabilities": [
    "rtsp_streaming",
    "snapshot_capture",
    "ai_inference",
    "motion_detection",
    "onvif_control"
  ],

  "semanticTags": [
    "カメラ",
    "RTSP",
    "映像監視",
    "AI検出"
  ]
}
```

### 3.3 userObject_detail/is801ParaclateCamera

```json
{
  "firmware": {
    "version": "N/A",
    "buildDate": null,
    "modules": ["RTSP", "ONVIF"]
  },

  "config": {
    "rtsp": {
      "mainStreamUrl": "rtsp://192.168.125.100:554/stream1",
      "subStreamUrl": "rtsp://192.168.125.100:554/stream2",
      "username": "admin",
      "password": "encrypted:xxxxx"
    },
    "inference": {
      "enabled": true,
      "preset": "detection_general",
      "confidenceThreshold": 0.5,
      "customThresholds": {
        "person": 0.6,
        "vehicle": 0.5,
        "animal": 0.4
      }
    },
    "context": {
      "name": "玄関カメラ",
      "location": "1F エントランス",
      "purpose": "来訪者監視",
      "notes": "夜間は赤外線モードに切り替わる"
    },
    "fitMode": "contain"
  },

  "status": {
    "online": true,
    "lastSeen": "2026-01-10T00:00:00Z"
  },

  "network": {
    "ip": "192.168.125.100",
    "gateway": "192.168.125.1",
    "subnet": "255.255.255.0"
  },

  "hardware": {
    "brand": "Hikvision",
    "model": "DS-2CD2143G2-I",
    "resolution": "2688x1520",
    "poe": true,
    "ptz": false,
    "irRange": "30m"
  },

  "parentServer": {
    "lacisId": "3022AABBCCDDEEFF0000",
    "ip": "192.168.125.246"
  }
}
```

---

## 4. araneaDeviceStates スキーマ

### 4.1 is22 CamServer

```json
{
  "lacisId": "3022AABBCCDDEEFF0000",
  "tid": "T2025120621041161827",
  "fid": ["0150", "0151"],
  "type": "aranea_ar-is22",

  "state": {
    "serverStatus": "running",
    "cameraStats": {
      "total": 8,
      "online": 7,
      "offline": 1,
      "error": 0
    },
    "inference": {
      "status": "processing",
      "queueSize": 2,
      "lastProcessedAt": "2026-01-10T05:50:00Z"
    },
    "storage": {
      "usagePercent": 45.2,
      "snapshotCount": 1250
    },
    "pollingCycle": {
      "currentCycle": 2000,
      "lastCycleAt": "2026-01-10T05:50:00Z",
      "avgCycleDurationMs": 8500
    }
  },

  "semanticTags": ["カメラ監視", "AI推論", "サマリーレポート"],

  "advert": {
    "observedAt": "2026-01-10T05:50:00Z"
  },

  "alive": true,
  "lastUpdatedAt": "2026-01-10T05:50:00Z"
}
```

### 4.2 is801 ParaclateCamera

```json
{
  "lacisId": "3801AABBCCDDEEFF0010",
  "tid": "T2025120621041161827",
  "fid": ["0150"],
  "type": "aranea_ar-is801",

  "state": {
    "connectionStatus": "online",
    "lastCapturedAt": "2026-01-10T05:50:00Z",
    "lastDetection": {
      "timestamp": "2026-01-10T05:48:30Z",
      "tags": ["person", "vehicle"],
      "confidence": 0.85
    },
    "streamHealth": {
      "fps": 15,
      "bitrate": 2048000,
      "resolution": "1920x1080",
      "codec": "H.264"
    },
    "errorCount24h": 0
  },

  "semanticTags": ["カメラ", "AI検出"],

  "advert": {
    "gatewayLacisId": "3022AABBCCDDEEFF0000",
    "observedAt": "2026-01-10T05:50:00Z"
  },

  "alive": true,
  "lastUpdatedAt": "2026-01-10T05:50:00Z"
}
```

---

## 5. deviceStateReport フォーマット

### 4.1 is22 CamServer レポート

```json
{
  "auth": {
    "tid": "T2025120621041161827",
    "lacisId": "3022AABBCCDDEEFF0000",
    "cic": "123456"
  },
  "report": {
    "type": "aranea_ar-is22",
    "state": {
      "serverStatus": "running",
      "cameraStats": {
        "total": 8,
        "online": 7,
        "offline": 1,
        "error": 0
      },
      "inference": {
        "status": "processing",
        "queueSize": 2
      }
    },
    "observedAt": "2026-01-10T05:50:00Z",
    "meta": {
      "firmwareVersion": "0.1.0",
      "heap": 174000000,
      "uptime": 86400
    }
  }
}
```

### 5.2 is22からのカメラバッチレポート

```json
{
  "auth": {
    "tid": "T2025120621041161827",
    "lacisId": "3022AABBCCDDEEFF0000",
    "cic": "123456"
  },
  "reports": [
    {
      "lacisId": "3801AABBCCDDEEFF0010",
      "type": "aranea_ar-is801",
      "state": {
        "connectionStatus": "online",
        "lastCapturedAt": "2026-01-10T05:50:00Z",
        "lastDetection": {
          "timestamp": "2026-01-10T05:48:30Z",
          "tags": ["person"],
          "confidence": 0.85
        }
      },
      "observedAt": "2026-01-10T05:50:00Z"
    },
    {
      "lacisId": "3801112233445566001",
      "type": "aranea_ar-is801",
      "state": {
        "connectionStatus": "offline",
        "errorCount24h": 3
      },
      "observedAt": "2026-01-10T05:50:00Z"
    }
  ]
}
```

---

## 6. Paraclate Summary スキーマ

### 6.1 Summary レポート

```json
{
  "lacisOath": {
    "lacisID": "3022AABBCCDDEEFF0000",
    "tid": "T2025120621041161827",
    "cic": "123456",
    "blessing": null
  },
  "summaryOverview": {
    "summaryID": "SUM-20260110-055000-XXXX",
    "firstDetectAt": "2026-01-10T04:00:00Z",
    "lastDetectAt": "2026-01-10T05:48:30Z",
    "detectedEvents": 15
  },
  "cameraContext": {
    "3801AABBCCDDEEFF0010": {
      "cameraName": "玄関カメラ",
      "cameraContext": "1F エントランス / 来訪者監視",
      "fid": "0150",
      "rid": null,
      "preset": "detection_general"
    },
    "3801112233445566001": {
      "cameraName": "駐車場カメラ",
      "cameraContext": "B1F 駐車場 / 車両監視",
      "fid": "0150",
      "rid": null,
      "preset": "detection_vehicle"
    }
  },
  "cameraDetection": [
    {
      "timestamp": "2026-01-10T05:48:30Z",
      "cameraLacisId": "3801AABBCCDDEEFF0010",
      "detectionDetail": {
        "tags": ["person"],
        "confidence": 0.85,
        "snapshotUrl": "gs://lacis-files/snapshots/xxxx.jpg"
      }
    }
  ]
}
```

---

## 7. 実装チェックリスト

### 7.1 mobes2.0側（araneaSDK提供）

- [x] userObject共通スキーマ（既存）
- [x] typeSettings/araneaDevice/aranea_ar-is22 登録 ✅ 2026-01-10
- [x] typeSettings/araneaDevice/aranea_ar-is21 登録 ✅ 2026-01-10
- [x] typeSettings/araneaDevice/aranea_ar-is801 登録 ✅ 2026-01-10
- [x] ProductType 022 (is22) 登録 ✅
- [x] ProductType 021 (is21) 登録 ✅
- [ ] ProductType 801 (is801) 登録

### 7.2 is22/is21側（実装責任）

- [x] AraneaRegister Phase 1 実装（is22）✅ CIC=605123
- [ ] AraneaRegister Phase 1 実装（is21）⏳ 共有ライブラリ化後
- [ ] userObject_detail/is22 構造実装
- [ ] userObject_detail/is21 構造実装
- [ ] userObject_detail/is801 構造実装
- [ ] deviceStateReport 送信実装
- [ ] Paraclate Summary 送信実装
- [ ] カメラLacisID生成ロジック (3801{MAC}{BrandCode})

---

## 8. MECE確認

本スキーマ定義は以下のMECE原則を満たす：

1. **共通スキーマ vs タイプ別スキーマ**: 明確に分離
2. **is22 vs is801**: 親子関係として明確に定義
3. **state vs config**: 読み取り専用状態と設定可能項目を分離
4. **ProductCode**: カメラブランドごとに一意に割り当て

本ドキュメントはaraneaSDK SCHEMA_SPEC.mdに完全準拠している。
