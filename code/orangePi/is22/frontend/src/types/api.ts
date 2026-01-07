export interface ApiResponse<T> {
  ok: boolean;
  data: T;
  error: string | null;
}

export type CameraFamily = 'tapo' | 'vigi' | 'nest' | 'axis' | 'hikvision' | 'dahua' | 'other' | 'unknown';

// =============================================================================
// ONVIF Extended Data Types
// =============================================================================

// GetScopes response data
export interface OnvifScopes {
  name?: string;           // e.g., "Tapo C100"
  hardware?: string;       // e.g., "C100"
  profile?: string;        // e.g., "Streaming"
  location?: string;       // e.g., "Hong Kong"
  type?: string;           // e.g., "NetworkVideoTransmitter"
  raw_scopes?: string[];   // 生のスコープURI配列
}

// GetNetworkInterfaces response data
export interface OnvifNetworkInterface {
  name: string;            // e.g., "eth0"
  mac_address: string;     // e.g., "3C:84:6A:73:18:3A"
  enabled?: boolean;
  ipv4?: {
    address: string;
    prefix_length: number;
    dhcp: boolean;
  };
}

// GetCapabilities response data
export interface OnvifCapabilities {
  analytics?: {
    supported: boolean;
    rule_support: boolean;
    analytics_module_support: boolean;
    xaddr?: string;
  };
  device?: {
    xaddr?: string;
    io_support?: boolean;
    io_input_connectors?: number;
    io_output_connectors?: number;
  };
  events?: {
    xaddr?: string;
    ws_subscription_support: boolean;
    ws_pull_point_support: boolean;
  };
  imaging?: {
    xaddr?: string;
  };
  media?: {
    xaddr?: string;
    rtp_multicast: boolean;
    rtp_tcp: boolean;
    rtp_rtsp_tcp: boolean;
  };
  ptz?: {
    xaddr?: string;
  };
}

export interface Camera {
  camera_id: string;
  name: string;
  location: string;
  floor: string | null;
  // === 管理用フィールド ===
  rid: string | null;           // 部屋ID/設置場所識別子
  rtsp_main: string | null;
  rtsp_sub: string | null;
  rtsp_username: string | null;
  rtsp_password: string | null;
  snapshot_url: string | null;
  family: CameraFamily;
  manufacturer: string | null;
  model: string | null;
  ip_address: string | null;
  mac_address: string | null;
  lacis_id: string | null;
  cic: string | null;           // 6桁登録コード
  enabled: boolean;
  polling_enabled: boolean;
  polling_interval_sec: number;
  suggest_policy_weight: number;
  camera_context: Record<string, unknown> | null;
  rotation: number;             // 表示回転角度 (0/90/180/270)
  fit_mode: 'fit' | 'trim';     // fit=画像全体収める, trim=カード埋める(デフォルト)
  fid: string | null;           // ファシリティID
  tid: string | null;           // テナントID
  sort_order: number;           // ドラッグ並べ替え用順序
  // === AI分析設定 ===
  preset_id: string | null;     // 利用するプリセットID
  preset_version: string | null; // プリセットバージョン
  ai_enabled: boolean;          // AI分析有効フラグ
  ai_interval_sec: number;      // AI分析間隔（秒）
  // === 閾値オーバーライド (migration 014) ===
  conf_override: number | null;  // confidence閾値 (0.20-0.80) - カメラ個別設定
  nms_threshold: number | null;  // NMS閾値 (0.30-0.60) - カメラ個別設定
  par_threshold: number | null;  // PAR閾値 (0.30-0.80) - カメラ個別設定
  // === ONVIF デバイス情報 ===
  serial_number: string | null;
  hardware_id: string | null;
  firmware_version: string | null;
  onvif_endpoint: string | null;
  // === ネットワーク情報 ===
  rtsp_port: number | null;
  http_port: number | null;
  onvif_port: number | null;
  // === ビデオ能力（メイン） ===
  resolution_main: string | null;
  codec_main: string | null;
  fps_main: number | null;
  bitrate_main: number | null;
  // === ビデオ能力（サブ） ===
  resolution_sub: string | null;
  codec_sub: string | null;
  fps_sub: number | null;
  bitrate_sub: number | null;
  // === PTZ能力 ===
  ptz_supported: boolean;
  ptz_continuous: boolean;
  ptz_absolute: boolean;
  ptz_relative: boolean;
  ptz_pan_range: { min: number; max: number } | null;
  ptz_tilt_range: { min: number; max: number } | null;
  ptz_zoom_range: { min: number; max: number } | null;
  ptz_presets: string[] | null;
  ptz_home_supported: boolean;
  // === 音声能力 ===
  audio_input_supported: boolean;
  audio_output_supported: boolean;
  audio_codec: string | null;
  // === ONVIFプロファイル ===
  onvif_profiles: Record<string, unknown>[] | null;
  // === ONVIF拡張データ ===
  onvif_scopes: OnvifScopes | null;
  onvif_network_interfaces: OnvifNetworkInterface[] | null;
  onvif_capabilities: OnvifCapabilities | null;
  // === 検出メタ情報 ===
  discovery_method: string | null;
  last_verified_at: string | null;
  last_rescan_at: string | null;
  deleted_at: string | null;    // ソフトデリート日時
  // === タイムスタンプ ===
  created_at: string;
  updated_at: string;
}

// カメラ更新リクエスト
export interface UpdateCameraRequest {
  // === 基本情報 ===
  name?: string;
  location?: string;
  floor?: string;
  rid?: string;
  // === ストリーム設定 ===
  rtsp_main?: string;
  rtsp_sub?: string;
  rtsp_username?: string;
  rtsp_password?: string;
  snapshot_url?: string;
  // === デバイス情報 ===
  family?: CameraFamily;
  manufacturer?: string;
  model?: string;
  ip_address?: string;
  mac_address?: string;
  lacis_id?: string;
  cic?: string;
  // === ポリシー ===
  enabled?: boolean;
  polling_enabled?: boolean;
  polling_interval_sec?: number;
  suggest_policy_weight?: number;
  // === カメラコンテキスト・表示 ===
  camera_context?: Record<string, unknown>;
  rotation?: number;
  fit_mode?: 'fit' | 'trim';
  sort_order?: number;
  // === 所属情報 ===
  fid?: string;
  tid?: string;
  // === AI分析設定 ===
  preset_id?: string;
  preset_version?: string;
  ai_enabled?: boolean;
  ai_interval_sec?: number;
  // === 閾値オーバーライド (migration 014) ===
  conf_override?: number | null;  // confidence閾値 (0.20-0.80) - null=プリセット値使用
  nms_threshold?: number | null;  // NMS閾値 (0.30-0.60) - null=プリセット値使用
  par_threshold?: number | null;  // PAR閾値 (0.30-0.80) - null=プリセット値使用
  // === ONVIF デバイス情報 ===
  serial_number?: string;
  hardware_id?: string;
  firmware_version?: string;
  onvif_endpoint?: string;
  // === ネットワーク情報 ===
  rtsp_port?: number;
  http_port?: number;
  onvif_port?: number;
  // === ビデオ能力（メイン） ===
  resolution_main?: string;
  codec_main?: string;
  fps_main?: number;
  bitrate_main?: number;
  // === ビデオ能力（サブ） ===
  resolution_sub?: string;
  codec_sub?: string;
  fps_sub?: number;
  bitrate_sub?: number;
  // === PTZ能力 ===
  ptz_supported?: boolean;
  ptz_continuous?: boolean;
  ptz_absolute?: boolean;
  ptz_relative?: boolean;
  ptz_pan_range?: { min: number; max: number };
  ptz_tilt_range?: { min: number; max: number };
  ptz_zoom_range?: { min: number; max: number };
  ptz_presets?: string[];
  ptz_home_supported?: boolean;
  // === 音声能力 ===
  audio_input_supported?: boolean;
  audio_output_supported?: boolean;
  audio_codec?: string;
  // === ONVIFプロファイル ===
  onvif_profiles?: Record<string, unknown>[];
  // === ONVIF拡張データ ===
  onvif_scopes?: OnvifScopes;
  onvif_network_interfaces?: OnvifNetworkInterface[];
  onvif_capabilities?: OnvifCapabilities;
  // === 検出メタ情報 ===
  discovery_method?: string;
}

// 認証テスト結果
export interface AuthTestResult {
  rtsp_success: boolean;
  onvif_success: boolean;
  model: string | null;
  message: string;
}

// 再スキャン結果
export interface RescanResult {
  found: boolean;
  old_ip: string | null;
  new_ip: string | null;
  updated: boolean;
}

// ソフトデリート結果
export interface SoftDeleteResult {
  camera_id: string;
  mac_address: string | null;
  deleted_at: string;
  recoverable: boolean;
}

// MAC復元リクエスト
export interface RestoreByMacRequest {
  mac_address: string;
}

export interface StreamInfo {
  camera_id: string;
  webrtc_url: string;
  hls_url: string;
  snapshot_url: string;
}

export interface Event {
  event_id: number;
  camera_id: string;
  frame_id: string;
  captured_at: string;
  primary_event: string;
  severity: number;
  tags: string[];
  unknown_flag: boolean;
  attributes: Record<string, unknown> | null;
  thumbnail_url: string | null;
  created_at: string;
}

// Alias for compatibility with components
export type EventWithAliases = Event & {
  started_at: string;  // alias for captured_at
  severity_max: number; // alias for severity
  tags_summary: string[]; // alias for tags
}

// Connection status for user feedback
export type ConnectionStatus =
  | 'not_tested'
  | 'success'
  | 'auth_required'
  | 'auth_failed'
  | 'timeout'
  | 'refused'
  | 'port_open_only'
  | 'unknown';

// Device classification
export type DeviceType =
  | 'unknown'
  | 'camera_confirmed'
  | 'camera_likely'
  | 'camera_possible'
  | 'nvr_likely'
  | 'network_device'
  | 'other_device';

// Suggested action for user
export type SuggestedAction =
  | 'none'
  | 'set_credentials'
  | 'retry_with_auth'
  | 'check_network'
  | 'manual_check'
  | 'ignore';

// Credential trial status (from backend scan)
export type CredentialStatus =
  | 'not_tried'   // クレデンシャル未設定/試行なし
  | 'success'     // 認証成功、モデル情報取得済み
  | 'failed';     // 全クレデンシャル試行失敗

// Detection reason - why we think this is a camera
export interface DetectionReason {
  oui_match: string | null;
  camera_ports: number[];
  onvif_status: ConnectionStatus;
  rtsp_status: ConnectionStatus;
  device_type: DeviceType;
  user_message: string;
  suggested_action: SuggestedAction;
}

// ScannedDevice - Backend API応答と完全一致
// GET /api/ipcamscan/devices の devices[] 要素
export interface ScannedDevice {
  device_id: string;
  ip: string;                    // Backend: "ip" (NOT "ip_address")
  mac: string | null;            // Backend: "mac" (NOT "mac_address")
  family: 'tapo' | 'vigi' | 'nest' | 'other' | 'unknown';  // lowercase from backend
  manufacturer: string | null;
  model: string | null;
  firmware: string | null;
  oui_vendor: string | null;
  hostnames: string[];
  open_ports: { port: number; status: string }[];  // Backend structure
  rtsp_uri: string | null;
  verified: boolean;
  status: 'discovered' | 'verifying' | 'verified' | 'rejected' | 'approved';
  first_seen: string;            // Backend: "first_seen" (NOT "discovered_at")
  last_seen: string;
  subnet: string;
  score: number;
  confidence: number;
  detection: DetectionReason | null;
  // Credential trial result (from Stage 7)
  credential_status: CredentialStatus;
  credential_username: string | null;  // 成功時のユーザー名
  credential_password: string | null;  // 成功時のパスワード
  // Tried credentials list (for T3-10 plain-text display)
  tried_credentials?: TriedCredentialResult[];
  // Category detail from backend
  category_detail?: DeviceCategoryDetail;
  // For category F: registered camera info
  registered_camera_id?: number;
  registered_camera_name?: string;
  ip_changed?: boolean;
}

// Scan log entry for detailed progress visibility (IS22_UI_DETAILED_SPEC Section 3.2)
export interface ScanLogEntry {
  timestamp: string;
  ip_address: string;
  event_type: 'arp_response' | 'port_open' | 'oui_match' | 'onvif_probe' | 'rtsp_probe' | 'device_classified' | 'credential_trial' | 'info' | 'warning' | 'error';
  message: string;
  port?: number;
  oui_vendor?: string;
}

// Scan job progress tracking (aligned with backend response)
export interface ScanJobSummary {
  total_ips: number;
  hosts_alive: number;
  cameras_found: number;
  cameras_verified: number;
}

export interface ScanJob {
  job_id: string;
  targets: string[];
  mode: string;
  ports: number[];
  timeout_ms: number;
  concurrency: number;
  status: 'queued' | 'running' | 'success' | 'failed';
  started_at: string | null;
  ended_at: string | null;
  summary: ScanJobSummary | null;
  created_at: string;
  // Frontend-only fields for progress display
  progress_percent?: number;
  current_phase?: string;
  devices_found?: number;
  logs?: ScanLogEntry[];
}

export interface SystemStatus {
  healthy: boolean;
  cpu_percent: number;
  memory_percent: number;
  active_modals: number;
  modal_budget_remaining: number;
  suggest_active: boolean;
}

export interface LeaseRequest {
  consumer: string;
  camera_id: string;
  quality: 'hd' | 'sd';
}

export interface LeaseResponse {
  lease_id: string;
  allowed_quality: 'hd' | 'sd';
  expires_at: string;
  stream_url: string;
}

// =============================================================================
// Subnet Settings & Device Categorization (is22_ScanModal_Enhancement_Spec.md)
// =============================================================================

// Trial credential for subnet authentication
export interface TrialCredential {
  username: string;
  password: string;
  priority: number;  // 試行順序 (1-10)
}

// Subnet with settings - GET /api/subnets
export interface Subnet {
  subnet_id: string;
  cidr: string;
  fid: string;
  tid?: string;  // テナントID（カメラに継承される）
  description: string;
  enabled: boolean;
  last_scanned_at: string | null;
  created_at: string;
  // Extended settings (optional, may not exist on older records)
  facility_name?: string;
  credentials?: TrialCredential[];
}

// Device category for scan results
// a: 登録済み - cameras テーブルに存在
// b: 登録可能 - カメラ確認済み + 認証OK + 未登録 ★強調
// c: 認証必要 - カメラ検出 + 認証未実施/失敗
// d: その他カメラ - カメラ可能性あり + 条件不足 (折りたたみ)
// e: 非カメラ - カメラでないデバイス (折りたたみ)
// f: 通信断・迷子カメラ - 登録済みだが応答なし / IPアドレス変更検出
export type DeviceCategory = 'a' | 'b' | 'c' | 'd' | 'e' | 'f';

// Device category detail (more specific classification)
export type DeviceCategoryDetail =
  | 'RegisteredAuthenticated'   // A: 登録済み・認証OK
  | 'RegisteredAuthIssue'       // A: 登録済み・認証要確認
  | 'Registrable'               // B: 登録可能
  | 'AuthRequired'              // C: 認証待ち
  | 'PossibleCamera'            // D: カメラ可能性あり（OUI一致）
  | 'NetworkEquipment'          // D: ネットワーク機器
  | 'IoTDevice'                 // D: IoTデバイス
  | 'UnknownDevice'             // D: 不明
  | 'NonCamera'                 // E: 非カメラ
  | 'LostConnection'            // F: 通信断
  | 'StrayChild';               // F: 迷子カメラ（IP変更検出）

// Category metadata for UI display
export interface CategoryMeta {
  id: DeviceCategory;
  label: string;
  description: string;
  bgClass: string;       // Tailwind background class
  collapsed: boolean;    // Default collapsed state
  userAction?: string;   // RT-09: ユーザーへの推奨アクション
}

// All category definitions
// Camscan_designers_review.md Item 2: カテゴリD は "その他ネットワークデバイス" に変更
// RT-09: ユーザーフレンドリーな文言に改善
export const DEVICE_CATEGORIES: CategoryMeta[] = [
  {
    id: 'a',
    label: '登録済みカメラ',
    description: '既にシステムに登録されているカメラです。正常に動作しています。',
    bgClass: 'bg-green-100',
    collapsed: false,
    userAction: '操作不要: 登録済みカメラは管理画面から確認・編集できます'
  },
  {
    id: 'b',
    label: '新規登録可能',
    description: 'カメラとして認識され、認証にも成功しました。すぐに登録できます。',
    bgClass: 'bg-blue-200 border-2 border-blue-500',
    collapsed: false,
    userAction: '推奨: チェックを入れて「登録」ボタンを押してください'
  },
  {
    id: 'c',
    label: '認証情報の設定が必要',
    description: 'カメラの可能性がありますが、ユーザー名・パスワードが必要です。',
    bgClass: 'bg-yellow-100',
    collapsed: false,
    userAction: '対応: サブネット設定でクレデンシャルを追加し、再スキャンしてください'
  },
  {
    id: 'd',
    label: 'ネットワーク機器',
    description: 'ルーターやスイッチなど、カメラ以外のネットワーク機器です。',
    bgClass: 'bg-gray-100',
    collapsed: true,
    userAction: '通常は無視: カメラでない場合は登録不要です'
  },
  {
    id: 'e',
    label: 'その他のデバイス',
    description: 'PCやスマートフォンなど、ネットワークに接続されているデバイスです。',
    bgClass: 'bg-gray-100',
    collapsed: true,
    userAction: '操作不要: カメラではないため登録対象外です'
  },
  {
    id: 'f',
    label: '要確認: 通信できません',
    description: '登録済みカメラですが、現在通信できません。電源やケーブルを確認してください。',
    bgClass: 'bg-red-100 border-2 border-red-400',
    collapsed: false,
    userAction: '要確認: 電源・ネットワークケーブル・IPアドレス設定を確認してください'
  },
];

// ScannedDevice with category (Frontend enriched)
export interface CategorizedDevice extends ScannedDevice {
  category: DeviceCategory;
  categoryDetail?: DeviceCategoryDetail;  // More specific category info
  isRegistered: boolean;  // cameras テーブルに存在するか
  registeredCameraName?: string;  // For category F: name of registered camera
  ipChanged?: boolean;  // For StrayChild detection
}

// =============================================================================
// Scan Progress Types (DynamicProgressCalculator)
// =============================================================================

// Scan stage for progress calculation
export type ScanStage =
  | 'HostDiscovery'   // ARP/Ping スキャン
  | 'PortScan'        // ポートスキャン
  | 'OuiLookup'       // OUI判定
  | 'OnvifProbe'      // ONVIF検出
  | 'RtspAuth'        // RTSP認証試行
  | 'CameraMatching'; // 登録済みカメラ照合

// Stage progress info
export interface StageProgress {
  stage: ScanStage;
  weight: number;         // Stage weight (0-100)
  actualHosts?: number;   // Number of hosts to process
  completedHosts: number; // Number of hosts completed
}

// Scan progress event from backend
export interface ScanProgressEvent {
  percent: number;
  currentStage?: ScanStage;
  stageDetails: StageProgress[];
}

// =============================================================================
// Tried Credential Types (for T3-10 plain-text display)
// =============================================================================

// Result of credential trial
export type CredentialResult = 'success' | 'failed' | 'timeout';

// Tried credential with result
export interface TriedCredentialResult {
  username: string;
  password: string;  // 平文表示（設計書 High #1 に準拠）
  result: CredentialResult;
}

// Camera status including PendingAuth
export type CameraStatus = 'active' | 'inactive' | 'pending_auth' | 'maintenance';

// =============================================================================
// AI Event Log (detection_logs) - MySQL永続化検知ログ
// =============================================================================

// Detection log entry from MySQL (GET /api/detection-logs)
export interface DetectionLog {
  log_id: number;
  tid: string;
  fid: string;
  camera_id: string;
  lacis_id: string | null;
  captured_at: string;
  analyzed_at: string;
  primary_event: string;
  severity: number;
  confidence: number;
  count_hint: number;
  tags: string[];
  unknown_flag: boolean;
  bboxes: BoundingBox[] | null;
  person_details: PersonDetail[] | null;
  suspicious: SuspiciousInfo | null;
  frame_diff: FrameDiff | null;
  loitering_detected: boolean;
  preset_id: string | null;
  preset_version: string | null;
  output_schema: string | null;
  context_applied: boolean;
  camera_context: CameraContext | null;
  is21_log: Record<string, unknown>;  // Raw IS21 response JSON
  image_path_local: string;           // Local filesystem path
  image_path_cloud: string | null;    // Cloud storage path (if uploaded)
  processing_ms: number | null;
  polling_cycle_id: string | null;
  schema_version: string;
  timings: ProcessingTimings | null;
  synced_to_bq: boolean;
  synced_at: string | null;
  created_at: string;
}

// Processing timing breakdown for bottleneck analysis
export interface ProcessingTimings {
  total_ms: number | null;
  snapshot_ms: number | null;
  is21_roundtrip_ms: number | null;
  is21_inference_ms: number | null;
  is21_yolo_ms: number | null;
  is21_par_ms: number | null;
  save_ms: number | null;
}

// Bounding box from AI analysis
export interface BoundingBox {
  x: number;
  y: number;
  w: number;
  h: number;
  label: string;
  confidence: number;
}

// Person attribute details
export interface PersonDetail {
  gender: string | null;
  age_group: string | null;
  clothing_color: string | null;
  posture: string | null;
}

// Suspicious behavior info
export interface SuspiciousInfo {
  score: number;
  reasons: string[];
}

// Frame diff analysis
export interface FrameDiff {
  has_change: boolean;
  change_percent: number;
  motion_regions: number;
}

// Camera context for AI analysis
export interface CameraContext {
  location?: string;
  purpose?: string;
  normal_occupancy?: string;
  time_context?: string;
  previous_frame?: PreviousFrameInfo;
}

// Previous frame info for diff analysis
export interface PreviousFrameInfo {
  captured_at: string;
  primary_event: string;
  count_hint: number;
  severity: number;
}

// Detection log statistics
export interface DetectionLogStats {
  total_logs: number;
  detected_count: number;
  detection_rate: number;
  by_severity: Record<string, number>;
  by_event_type: Record<string, number>;
  pending_bq_sync: number;
  last_detection_at: string | null;
}

// Patrol feedback item (for UI "動いてる安心感")
export interface PatrolFeedback {
  camera_id: string;
  camera_name: string;
  timestamp: string;
  primary_event: string | null;
  severity: number | null;
  is_detection: boolean;
}
