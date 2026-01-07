/**
 * SdmSettingsTab - Google/Nest (SDM) Integration Settings
 *
 * Wizard-based UI for configuring SDM (Smart Device Management) integration.
 * Follows GoogleDevices_settings_wizard_spec.md specification.
 *
 * Steps:
 * 1. Overview & Prerequisites (フールプルーフ前提チェック)
 * 2. SDM Settings Input (SDM設定入力)
 * 3. Authorization Code (認可コード取得)
 * 4. Device List (デバイス一覧取得)
 * 5. Registration Dialog (登録ダイアログ)
 * 6. Connection Confirmation (接続確認)
 */

import { useState, useCallback, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  CheckCircle2,
  XCircle,
  AlertTriangle,
  RefreshCw,
  ExternalLink,
  Copy,
  ChevronRight,
  ChevronLeft,
  ChevronDown,
  ChevronUp,
  Check,
  Info,
  HelpCircle,
  Bell,
  Settings2,
  Play,
  Image,
} from "lucide-react"
import { API_BASE_URL } from "@/lib/config"
import type {
  SdmConfigResponse,
  SdmConfigSaveRequest,
  SdmDevice,
  SdmRegisterDeviceRequest,
  Go2rtcVersionResponse,
  Camera,
} from "@/types/api"

// =============================================================================
// Types
// =============================================================================

type WizardStep = 1 | 2 | 3 | 4 | 5 | 6

interface StepStatus {
  completed: boolean
  accessible: boolean
}

// =============================================================================
// Constants
// =============================================================================

const STEP_TITLES: Record<WizardStep, string> = {
  1: "準備確認",
  2: "SDM設定",
  3: "認可コード",
  4: "デバイス一覧",
  5: "カメラ登録",
  6: "接続確認",
}

// =============================================================================
// Main Component
// =============================================================================

export function SdmSettingsTab() {
  // Current wizard step
  const [currentStep, setCurrentStep] = useState<WizardStep>(1)

  // SDM config state
  const [config, setConfig] = useState<SdmConfigResponse | null>(null)
  const [configLoading, setConfigLoading] = useState(false)

  // Step 1: Prerequisites checkboxes
  const [prereqChecks, setPrereqChecks] = useState({
    hasProjectId: false,
    hasClientId: false,
    hasPaidFee: false,
    hasDoorbellInHome: false,
  })
  // Step 1: Expanded guide
  const [showSetupGuide, setShowSetupGuide] = useState(false)

  // Step 2: Form values
  const [formValues, setFormValues] = useState({
    project_id: "",
    project_number: "",
    enterprise_project_id: "",
    client_id: "",
    client_secret: "",
    refresh_token: "",
  })
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [saveSuccess, setSaveSuccess] = useState(false)

  // Step 3: Auth code
  const [authCode, setAuthCode] = useState("")
  const [exchanging, setExchanging] = useState(false)
  const [exchangeError, setExchangeError] = useState<string | null>(null)
  const [exchangeSuccess, setExchangeSuccess] = useState(false)

  // Step 4: Devices
  const [devices, setDevices] = useState<SdmDevice[]>([])
  const [devicesLoading, setDevicesLoading] = useState(false)
  const [devicesError, setDevicesError] = useState<string | null>(null)

  // Step 5: Registration
  const [selectedDevice, setSelectedDevice] = useState<SdmDevice | null>(null)
  const [registerForm, setRegisterForm] = useState({
    camera_id: "",
    name: "",
    location: "",
    fid: "",
    tid: "",
  })
  const [registering, setRegistering] = useState(false)
  const [registerError, setRegisterError] = useState<string | null>(null)
  const [registeredCamera, setRegisteredCamera] = useState<Camera | null>(null)

  // Step 6: Connection confirmation
  const [snapshotUrl, setSnapshotUrl] = useState<string | null>(null)
  const [snapshotLoading, setSnapshotLoading] = useState(false)
  const [snapshotError, setSnapshotError] = useState<string | null>(null)

  // Advanced settings panel
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [go2rtcVersion, setGo2rtcVersion] = useState<Go2rtcVersionResponse | null>(null)
  const [versionLoading, setVersionLoading] = useState(false)
  const [tokenTestResult, setTokenTestResult] = useState<{ ok: boolean; error?: string } | null>(null)
  const [tokenTesting, setTokenTesting] = useState(false)

  // Help panel
  const [showHelp, setShowHelp] = useState(false)

  // =============================================================================
  // API Functions
  // =============================================================================

  const fetchConfig = useCallback(async () => {
    setConfigLoading(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/sdm/config`)
      if (resp.ok) {
        const json = await resp.json()
        const data = json.data || json
        setConfig(data)
        // Pre-fill form with existing values
        if (data.project_id) setFormValues(prev => ({ ...prev, project_id: data.project_id }))
        if (data.project_number) setFormValues(prev => ({ ...prev, project_number: data.project_number }))
        if (data.enterprise_project_id) setFormValues(prev => ({ ...prev, enterprise_project_id: data.enterprise_project_id }))
        if (data.client_id) setFormValues(prev => ({ ...prev, client_id: data.client_id }))
      }
    } catch (error) {
      console.error("Failed to fetch SDM config:", error)
    } finally {
      setConfigLoading(false)
    }
  }, [])

  const saveConfig = async () => {
    setSaving(true)
    setSaveError(null)
    setSaveSuccess(false)
    try {
      const payload: SdmConfigSaveRequest = {
        project_id: formValues.project_id,
        project_number: formValues.project_number || undefined,
        enterprise_project_id: formValues.enterprise_project_id,
        client_id: formValues.client_id,
        client_secret: formValues.client_secret,
        refresh_token: formValues.refresh_token || undefined,
      }
      const resp = await fetch(`${API_BASE_URL}/api/sdm/config`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      })
      if (!resp.ok) {
        const json = await resp.json()
        throw new Error(json.error || `HTTP ${resp.status}`)
      }
      setSaveSuccess(true)
      await fetchConfig()
      // Clear sensitive fields after save
      setFormValues(prev => ({ ...prev, client_secret: "", refresh_token: "" }))
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : "保存に失敗しました")
    } finally {
      setSaving(false)
    }
  }

  const exchangeAuthCode = async () => {
    setExchanging(true)
    setExchangeError(null)
    setExchangeSuccess(false)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/sdm/exchange-code`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ auth_code: authCode }),
      })
      if (!resp.ok) {
        const json = await resp.json()
        const errorMsg = json.error || `HTTP ${resp.status}`
        // User-friendly error messages
        if (errorMsg.includes("invalid_grant") || resp.status === 401) {
          throw new Error("コードが古いか無効です。もう一度URLを開いてやり直してください")
        }
        if (errorMsg.includes("redirect_uri")) {
          throw new Error("Redirect URI が違います。`https://www.google.com` を使ってください")
        }
        throw new Error(errorMsg)
      }
      setExchangeSuccess(true)
      setAuthCode("")
      await fetchConfig()
    } catch (error) {
      setExchangeError(error instanceof Error ? error.message : "コード交換に失敗しました")
    } finally {
      setExchanging(false)
    }
  }

  const fetchDevices = async () => {
    setDevicesLoading(true)
    setDevicesError(null)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/sdm/devices`)
      if (!resp.ok) {
        const json = await resp.json()
        if (resp.status === 401) {
          throw new Error("認可が無効です。再認可が必要です")
        }
        throw new Error(json.error || `HTTP ${resp.status}`)
      }
      const json = await resp.json()
      const data = json.data || json
      setDevices(data.devices || [])
    } catch (error) {
      setDevicesError(error instanceof Error ? error.message : "デバイス取得に失敗しました")
    } finally {
      setDevicesLoading(false)
    }
  }

  const registerDevice = async () => {
    if (!selectedDevice) return
    setRegistering(true)
    setRegisterError(null)
    try {
      const payload: SdmRegisterDeviceRequest = {
        camera_id: registerForm.camera_id,
        name: registerForm.name,
        location: registerForm.location,
        fid: registerForm.fid,
        tid: registerForm.tid,
      }
      const resp = await fetch(`${API_BASE_URL}/api/sdm/devices/${encodeURIComponent(selectedDevice.sdm_device_id)}/register`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      })
      if (!resp.ok) {
        const json = await resp.json()
        if (resp.status === 401) {
          throw new Error("再認可が必要です")
        }
        if (resp.status === 409) {
          throw new Error("camera_id が既に使用されています")
        }
        throw new Error(json.error || `HTTP ${resp.status}`)
      }
      const json = await resp.json()
      setRegisteredCamera(json.data || json)
      setCurrentStep(6)
    } catch (error) {
      setRegisterError(error instanceof Error ? error.message : "登録に失敗しました")
    } finally {
      setRegistering(false)
    }
  }

  const fetchSnapshot = async () => {
    if (!registeredCamera) return
    setSnapshotLoading(true)
    setSnapshotError(null)
    try {
      // Use go2rtc frame endpoint
      const url = `${API_BASE_URL}/api/go2rtc/frame.jpeg?src=${registeredCamera.camera_id}&t=${Date.now()}`
      setSnapshotUrl(url)
    } catch (error) {
      setSnapshotError(error instanceof Error ? error.message : "スナップショット取得に失敗しました")
    } finally {
      setSnapshotLoading(false)
    }
  }

  const checkGo2rtcVersion = async () => {
    setVersionLoading(true)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/sdm/test/go2rtc-version`)
      if (resp.ok) {
        const json = await resp.json()
        setGo2rtcVersion(json.data || json)
      }
    } catch (error) {
      console.error("Failed to check go2rtc version:", error)
    } finally {
      setVersionLoading(false)
    }
  }

  const testToken = async () => {
    setTokenTesting(true)
    setTokenTestResult(null)
    try {
      const resp = await fetch(`${API_BASE_URL}/api/sdm/test/token`)
      const json = await resp.json()
      setTokenTestResult(json.data || json)
    } catch (error) {
      setTokenTestResult({ ok: false, error: "接続に失敗しました" })
    } finally {
      setTokenTesting(false)
    }
  }

  // =============================================================================
  // Effects
  // =============================================================================

  useEffect(() => {
    fetchConfig()
  }, [fetchConfig])

  useEffect(() => {
    if (currentStep === 6 && registeredCamera) {
      fetchSnapshot()
    }
  }, [currentStep, registeredCamera])

  // Auto-populate registration form when device is selected
  useEffect(() => {
    if (selectedDevice) {
      const shortId = selectedDevice.sdm_device_id.split("/").pop()?.slice(-8) || ""
      setRegisterForm({
        camera_id: `nest-${shortId}`.toLowerCase(),
        name: selectedDevice.name || "Nest Doorbell",
        location: selectedDevice.room || "",
        fid: "",
        tid: "",
      })
    }
  }, [selectedDevice])

  // =============================================================================
  // Step Validation
  // =============================================================================

  const getStepStatus = (step: WizardStep): StepStatus => {
    switch (step) {
      case 1:
        return { completed: Object.values(prereqChecks).every(Boolean), accessible: true }
      case 2:
        return {
          completed: config?.configured || false,
          accessible: Object.values(prereqChecks).every(Boolean),
        }
      case 3:
        return {
          completed: config?.has_refresh_token || false,
          accessible: config?.configured || false,
        }
      case 4:
        return {
          completed: devices.length > 0,
          accessible: config?.has_refresh_token || false,
        }
      case 5:
        return {
          completed: registeredCamera !== null,
          accessible: devices.length > 0,
        }
      case 6:
        return {
          completed: snapshotUrl !== null && !snapshotError,
          accessible: registeredCamera !== null,
        }
      default:
        return { completed: false, accessible: false }
    }
  }

  const canProceed = (): boolean => {
    const status = getStepStatus(currentStep)
    return status.completed
  }

  // =============================================================================
  // Generate Auth URL
  // =============================================================================

  const generateAuthUrl = (): string => {
    if (!config?.client_id || !config?.project_id) return ""
    const params = new URLSearchParams({
      client_id: config.client_id,
      redirect_uri: "https://www.google.com",
      response_type: "code",
      scope: "https://www.googleapis.com/auth/sdm.service",
      access_type: "offline",
      prompt: "consent",
    })
    return `https://nestservices.google.com/partnerconnections/${config.project_id}/auth?${params.toString()}`
  }

  // =============================================================================
  // Navigation
  // =============================================================================

  const goToStep = (step: WizardStep) => {
    const status = getStepStatus(step)
    if (status.accessible) {
      setCurrentStep(step)
    }
  }

  const goNext = () => {
    if (currentStep < 6 && canProceed()) {
      setCurrentStep((prev) => (prev + 1) as WizardStep)
    }
  }

  const goBack = () => {
    if (currentStep > 1) {
      setCurrentStep((prev) => (prev - 1) as WizardStep)
    }
  }

  // =============================================================================
  // Render Steps
  // =============================================================================

  const renderStep1 = () => (
    <div className="space-y-4">
      {/* 最重要：アカウントの説明 */}
      <div className="bg-red-50 border-2 border-red-300 rounded-lg p-4">
        <h4 className="font-bold text-red-800 mb-2 flex items-center gap-2">
          <AlertTriangle className="h-5 w-5" />
          最初に確認：使用するGoogleアカウントについて
        </h4>
        <div className="text-sm text-red-700 space-y-2">
          <p className="font-bold">
            この設定では、<span className="underline">すべての手順で同じGoogleアカウント</span>を使います。
          </p>
          <p>
            使用するアカウント ={" "}
            <strong>Google Home アプリでドアベルを登録したアカウント</strong>
          </p>
          <div className="bg-white border border-red-200 rounded p-2 mt-2">
            <p className="text-xs font-bold text-red-800 mb-1">⚠️ 注意：家族メンバーのアカウントは使えません</p>
            <p className="text-xs">
              「家」に招待されているだけのアカウントではダメです。<br />
              ドアベルを<strong>自分で追加した人</strong>（または家のオーナー）のアカウントが必要です。<br />
              Google Home アプリで「デバイス設定」→「リンク済みアカウント」を確認してください。
            </p>
          </div>
          <div className="bg-white border border-red-200 rounded p-2">
            <p className="text-xs font-bold text-red-800 mb-1">⚠️ 会社のGoogleアカウント（Workspace）は避けてください</p>
            <p className="text-xs">
              @gmail.com の個人アカウントを推奨します。会社アカウントでは失敗することがあります。
            </p>
          </div>
        </div>
      </div>

      {/* 概要 */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <h4 className="font-medium text-blue-800 mb-2 flex items-center gap-2">
          <Info className="h-4 w-4" />
          この設定で何ができるようになるか
        </h4>
        <p className="text-sm text-blue-700">
          Google Nest Doorbell の映像を、既存の防犯カメラと同じ画面で監視できるようになります。
          AI分析（人検知など）も使えます。
        </p>
      </div>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">準備チェックリスト（順番に確認）</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-xs text-muted-foreground">
            <strong>上から順番に</strong>確認してください。1つ目が終わっていないと2つ目に進めません。
          </p>

          {/* 1. Google Home でドアベル登録（最初！） */}
          <div className="border-2 border-purple-300 rounded-lg p-3 bg-purple-50">
            <label className="flex items-start gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={prereqChecks.hasDoorbellInHome}
                onChange={(e) => setPrereqChecks(prev => ({ ...prev, hasDoorbellInHome: e.target.checked }))}
                className="mt-1 h-5 w-5"
              />
              <div>
                <span className="text-sm font-bold text-purple-800">
                  ① Google Home アプリでドアベルがセットアップ済み
                </span>
                <p className="text-xs text-purple-700 mt-1">
                  <strong>これが最初！</strong> ドアベルが物理的にWi-Fiに接続され、
                  Google Home アプリに表示されている必要があります。
                </p>
                <p className="text-xs text-gray-600 mt-1">
                  まだの場合 → スマホで「Google Home」アプリを開く →「+」→「デバイスのセットアップ」
                </p>
              </div>
            </label>
          </div>

          {/* 2. $5 支払い */}
          <label className="flex items-start gap-3 cursor-pointer border rounded-lg p-3">
            <input
              type="checkbox"
              checked={prereqChecks.hasPaidFee}
              onChange={(e) => setPrereqChecks(prev => ({ ...prev, hasPaidFee: e.target.checked }))}
              className="mt-1 h-5 w-5"
            />
            <div>
              <span className="text-sm font-bold">
                ② Device Access に <span className="text-red-600">$5（約750円）</span> を支払い済み
              </span>
              <p className="text-xs text-muted-foreground mt-1">
                Googleへの一回限りの登録料です。クレジットカードが必要です。<br />
                <a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                  Device Access Console <ExternalLink className="h-3 w-3 inline" />
                </a>
                {" "}を開き、①と<strong>同じアカウント</strong>でログインして支払います。
              </p>
              <p className="text-xs text-orange-600 mt-1">
                ※支払い後、有効になるまで数分〜数時間かかることがあります
              </p>
            </div>
          </label>

          {/* 3. Device Access Console でプロジェクト作成 */}
          <label className="flex items-start gap-3 cursor-pointer border rounded-lg p-3">
            <input
              type="checkbox"
              checked={prereqChecks.hasProjectId}
              onChange={(e) => setPrereqChecks(prev => ({ ...prev, hasProjectId: e.target.checked }))}
              className="mt-1 h-5 w-5"
            />
            <div>
              <span className="text-sm font-bold">
                ③ Device Access Console で「プロジェクト」を作成済み
              </span>
              <p className="text-xs text-muted-foreground mt-1">
                <a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                  Device Access Console <ExternalLink className="h-3 w-3 inline" />
                </a>
                {" "}で「+ Create project」をクリックして作成します。
              </p>
              <div className="bg-gray-100 rounded p-2 mt-2 text-xs">
                <p className="font-bold text-gray-700">💡「プロジェクト名」って何を入れるの？</p>
                <p className="text-gray-600">
                  自分の名前ではありません。この設定に付ける「ラベル」です。<br />
                  例: <code className="bg-white px-1">nest-doorbell</code> や <code className="bg-white px-1">home-camera</code> など、<br />
                  英語で適当な名前を付ければOKです。後から変えられます。
                </p>
              </div>
            </div>
          </label>

          {/* 4. Google Cloud Console で OAuth Client 作成 */}
          <label className="flex items-start gap-3 cursor-pointer border rounded-lg p-3">
            <input
              type="checkbox"
              checked={prereqChecks.hasClientId}
              onChange={(e) => setPrereqChecks(prev => ({ ...prev, hasClientId: e.target.checked }))}
              className="mt-1 h-5 w-5"
            />
            <div>
              <span className="text-sm font-bold">
                ④ Google Cloud Console で OAuth Client を作成済み
              </span>
              <p className="text-xs text-muted-foreground mt-1">
                <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                  Google Cloud Console <ExternalLink className="h-3 w-3 inline" />
                </a>
                {" "}で「OAuth 2.0 クライアント ID」を作成します。
                <br />
                ①と<strong>同じアカウント</strong>でログインしてください。
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                作成後、<strong>クライアントID</strong> と <strong>クライアントシークレット</strong> をメモしておきます。
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                Redirect URI は <code className="bg-muted px-1 rounded">https://www.google.com</code> を設定します。
              </p>
            </div>
          </label>
        </CardContent>
      </Card>

      {/* 準備手順ガイド（展開式） */}
      <Card>
        <CardHeader className="pb-2">
          <button
            type="button"
            onClick={() => setShowSetupGuide(!showSetupGuide)}
            className="flex items-center justify-between w-full text-left"
          >
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <HelpCircle className="h-4 w-4 text-blue-600" />
              詳しい手順を見る（初めての方・迷った方はこちら）
            </CardTitle>
            {showSetupGuide ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
          </button>
        </CardHeader>
        {showSetupGuide && (
          <CardContent className="space-y-4 text-sm">
            {/* 重要な注意 */}
            <div className="bg-red-100 border border-red-300 rounded p-3">
              <p className="text-xs font-bold text-red-800">
                🔴 重要：以下のすべての手順で「同じGoogleアカウント」を使ってください！
              </p>
              <p className="text-xs text-red-700 mt-1">
                途中で違うアカウントでログインすると、ドアベルが見つからなくなります。
              </p>
            </div>

            {/* ステップ1: Google Home（最初にやる） */}
            <div className="border-l-4 border-purple-400 pl-3 space-y-2">
              <h5 className="font-bold text-purple-800">手順①　Google Home でドアベルをセットアップ（最初にやる）</h5>
              <p className="text-xs text-gray-600 mb-2">
                すでにセットアップ済みで、アプリにドアベルが表示されていればスキップしてOKです。
              </p>
              <ol className="list-decimal list-inside space-y-1 text-xs text-gray-700">
                <li>スマホで「<strong>Google Home</strong>」アプリを開く（なければインストール）</li>
                <li>使用するGoogleアカウントでログイン</li>
                <li>左上「+」→「デバイスのセットアップ」→「新しいデバイス」</li>
                <li>画面の指示に従って Nest Doorbell をWi-Fiに接続</li>
                <li>完了したら、アプリのトップ画面にドアベルが表示されることを確認</li>
              </ol>
              <div className="bg-purple-50 p-2 rounded text-xs mt-2">
                <strong>確認方法：</strong> Google Home アプリを開いて、ドアベルのカメラ映像が見られればOKです。
              </div>
            </div>

            {/* ステップ2: Device Access Console（$5 支払い＋プロジェクト作成） */}
            <div className="border-l-4 border-green-400 pl-3 space-y-2">
              <h5 className="font-bold text-green-800">手順②　Device Access Console で $5 支払い＋プロジェクト作成</h5>
              <ol className="list-decimal list-inside space-y-1 text-xs text-gray-700">
                <li>下のリンクをクリック →{" "}
                  <a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                    Device Access Console を開く
                  </a>
                </li>
                <li><strong>①と同じGoogleアカウント</strong>でログイン</li>
                <li>「Go to the Device Access Console」をクリック</li>
                <li><span className="text-red-600 font-bold">$5の支払い</span>（クレジットカード）→ 支払い完了まで進む</li>
                <li className="text-orange-600">※支払い後、数分〜数時間待つ必要がある場合があります</li>
                <li>「+ Create project」をクリック</li>
                <li>プロジェクト名を入力（例: <code className="bg-gray-100 px-1">nest-doorbell</code>）<br />
                  <span className="text-gray-500">※自分の名前ではなく、適当な英語の名前でOK</span>
                </li>
                <li>OAuth Client ID の欄は、次の手順③で作成してから入力します（一旦空欄でOK）</li>
                <li>「Enable」→「Create project」</li>
                <li>作成されたら、画面に表示される <strong>Project ID</strong> をメモ</li>
                <li>「Project Info」タブをクリック → <strong>Enterprise ID</strong>（enterprises/で始まる）をメモ</li>
              </ol>
            </div>

            {/* ステップ3: Google Cloud Console */}
            <div className="border-l-4 border-blue-400 pl-3 space-y-2">
              <h5 className="font-bold text-blue-800">手順③　Google Cloud Console で OAuth Client を作る</h5>
              <ol className="list-decimal list-inside space-y-1 text-xs text-gray-700">
                <li>下のリンクをクリック →{" "}
                  <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                    Google Cloud Console を開く
                  </a>
                </li>
                <li><strong>①②と同じGoogleアカウント</strong>でログイン</li>
                <li>画面上部の「プロジェクトを選択」→「新しいプロジェクト」をクリック</li>
                <li>プロジェクト名を入力（例: <code className="bg-gray-100 px-1">nest-doorbell</code>）→「作成」<br />
                  <span className="text-gray-500">※②のDevice Accessと同じ名前にしなくてもOK</span>
                </li>
                <li>左メニュー「認証情報」→ 上部「+ 認証情報を作成」→「OAuth クライアント ID」</li>
                <li>「同意画面を設定」を求められたら →「外部」を選択 →「作成」</li>
                <li>アプリ名を入力（適当でOK）、メールアドレスを2箇所に入力 →「保存して次へ」を3回クリック</li>
                <li>再度「認証情報」→「+ 認証情報を作成」→「OAuth クライアント ID」</li>
                <li>アプリケーションの種類:「<strong>ウェブ アプリケーション</strong>」を選択</li>
                <li>「承認済みのリダイレクト URI」に <code className="bg-gray-100 px-1">https://www.google.com</code> を追加</li>
                <li>「作成」→ 表示された <strong>クライアントID</strong> と <strong>クライアントシークレット</strong> をメモ</li>
              </ol>
            </div>

            {/* ステップ4: Device Access に Client ID を紐付け */}
            <div className="border-l-4 border-orange-400 pl-3 space-y-2">
              <h5 className="font-bold text-orange-800">手順④　Device Access にクライアントIDを紐付ける</h5>
              <ol className="list-decimal list-inside space-y-1 text-xs text-gray-700">
                <li><a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                    Device Access Console
                  </a> に戻る
                </li>
                <li>作成したプロジェクトをクリック</li>
                <li>「OAuth Client ID」の欄に、③でメモした<strong>クライアントID</strong>を貼り付け</li>
                <li>「Save」をクリック</li>
              </ol>
            </div>

            <div className="bg-green-50 border border-green-300 rounded p-3 text-xs">
              <p className="font-bold text-green-800 mb-2">✅ 準備完了！次のステップで入力する値：</p>
              <ul className="space-y-1 text-green-700">
                <li>・<strong>Project ID</strong>: 手順②で作成したプロジェクト名（例: nest-doorbell）</li>
                <li>・<strong>Enterprise ID</strong>: enterprises/○○○ で始まる長い文字列</li>
                <li>・<strong>クライアントID</strong>: ○○○.apps.googleusercontent.com</li>
                <li>・<strong>クライアントシークレット</strong>: GOCSPX-○○○</li>
              </ul>
            </div>
          </CardContent>
        )}
      </Card>
    </div>
  )

  const renderStep2 = () => (
    <div className="space-y-4">
      {/* 入力ガイド */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-3">
        <p className="text-xs text-blue-800">
          <Info className="h-3 w-3 inline mr-1" />
          前のステップでメモした値を入力してください。わからない場合は「戻る」を押して準備手順を確認できます。
        </p>
      </div>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">SDM 設定</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="project_id">Project ID *</Label>
            <Input
              id="project_id"
              placeholder="例: my-nest-project"
              value={formValues.project_id}
              onChange={(e) => setFormValues(prev => ({ ...prev, project_id: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              <strong>どこにある？</strong>{" "}
              <a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                Device Access Console
              </a>
              {" "}→ 作成したプロジェクト名をクリック → 画面上部に表示
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="project_number">Project Number (任意)</Label>
            <Input
              id="project_number"
              placeholder="例: 123456789012"
              value={formValues.project_number}
              onChange={(e) => setFormValues(prev => ({ ...prev, project_number: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              空欄でもOKです
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="enterprise_project_id">Enterprise Project ID *</Label>
            <Input
              id="enterprise_project_id"
              placeholder="例: enterprises/abc123-def456-7890"
              value={formValues.enterprise_project_id}
              onChange={(e) => setFormValues(prev => ({ ...prev, enterprise_project_id: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              <strong>どこにある？</strong>{" "}
              <a href="https://console.nest.google.com/device-access" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                Device Access Console
              </a>
              {" "}→ プロジェクトをクリック →「Project Info」タブ →「Enterprise ID」の値（enterprises/で始まる）
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="client_id">OAuth Client ID *</Label>
            <Input
              id="client_id"
              placeholder="例: 123456789-abcdefg.apps.googleusercontent.com"
              value={formValues.client_id}
              onChange={(e) => setFormValues(prev => ({ ...prev, client_id: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              <strong>どこにある？</strong>{" "}
              <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                Google Cloud Console
              </a>
              {" "}→「認証情報」→ OAuth 2.0 クライアント ID の行をクリック →「クライアント ID」の値
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="client_secret">
              OAuth Client Secret *
              {config?.has_client_secret && (
                <Badge variant="secondary" className="ml-2 text-xs">保存済み</Badge>
              )}
            </Label>
            <Input
              id="client_secret"
              type="password"
              placeholder={config?.has_client_secret ? "変更する場合のみ入力" : "Client Secret を入力"}
              value={formValues.client_secret}
              onChange={(e) => setFormValues(prev => ({ ...prev, client_secret: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              <strong>どこにある？</strong>{" "}
              <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noopener noreferrer" className="text-blue-600 underline">
                Google Cloud Console
              </a>
              {" "}→「認証情報」→ OAuth 2.0 クライアント ID の行をクリック →「クライアント シークレット」の値
              {config?.has_client_secret && "（既に保存済みの場合は空欄でOK）"}
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="refresh_token">
              Refresh Token (任意)
              {config?.has_refresh_token && (
                <Badge variant="secondary" className="ml-2 text-xs">保存済み</Badge>
              )}
            </Label>
            <Input
              id="refresh_token"
              type="password"
              placeholder="既に取得済みの場合のみ入力"
              value={formValues.refresh_token}
              onChange={(e) => setFormValues(prev => ({ ...prev, refresh_token: e.target.value }))}
            />
            <p className="text-xs text-muted-foreground">
              Step 3 で取得するか、事前に取得済みの場合はここで入力できます
            </p>
          </div>

          {saveError && (
            <div className="bg-red-50 border border-red-200 rounded p-2 text-sm text-red-700">
              {saveError}
            </div>
          )}

          {saveSuccess && (
            <div className="bg-green-50 border border-green-200 rounded p-2 text-sm text-green-700 flex items-center gap-2">
              <CheckCircle2 className="h-4 w-4" />
              保存しました
            </div>
          )}

          <Button
            onClick={saveConfig}
            disabled={saving || !formValues.project_id || !formValues.enterprise_project_id || !formValues.client_id || (!formValues.client_secret && !config?.has_client_secret)}
          >
            {saving && <RefreshCw className="h-4 w-4 mr-2 animate-spin" />}
            設定を保存
          </Button>
        </CardContent>
      </Card>
    </div>
  )

  const renderStep3 = () => {
    const authUrl = generateAuthUrl()

    return (
      <div className="space-y-4">
        {config?.has_refresh_token ? (
          <div className="bg-green-50 border border-green-200 rounded-lg p-4">
            <div className="flex items-center gap-2 text-green-700">
              <CheckCircle2 className="h-5 w-5" />
              <span className="font-medium">認可済み</span>
            </div>
            <p className="text-sm text-green-600 mt-1">
              refresh_token が保存されています。次のステップに進めます。
            </p>
          </div>
        ) : (
          <>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium">認可URLを開く</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <p className="text-sm text-muted-foreground">
                  以下のURLをブラウザで開き、Google アカウントでログインして許可してください。
                </p>

                <div className="bg-muted p-2 rounded text-xs font-mono break-all">
                  {authUrl || "設定を保存してください"}
                </div>

                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => navigator.clipboard.writeText(authUrl)}
                    disabled={!authUrl}
                  >
                    <Copy className="h-4 w-4 mr-1" />
                    コピー
                  </Button>
                  <Button
                    size="sm"
                    onClick={() => window.open(authUrl, "_blank")}
                    disabled={!authUrl}
                  >
                    <ExternalLink className="h-4 w-4 mr-1" />
                    ブラウザで開く
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium">認可コードを貼り付け</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="bg-blue-50 border border-blue-200 rounded p-3 text-sm text-blue-800 space-y-3">
                  <p className="font-bold">手順：</p>
                  <ol className="list-decimal list-inside space-y-2">
                    <li>上の「ブラウザで開く」ボタンをクリック</li>
                    <li>Google アカウントでログイン（個人Gmailを推奨）</li>
                    <li>「アクセスを許可」をクリック</li>
                    <li>画面が真っ白になり、ブラウザの<strong>アドレスバー（上の方）</strong>を見る</li>
                  </ol>

                  <div className="bg-white border-2 border-blue-300 rounded p-2 mt-2">
                    <p className="text-xs text-gray-600 mb-1">アドレスバーに表示される例：</p>
                    <p className="font-mono text-xs break-all">
                      https://www.google.com/?code=<span className="bg-yellow-200 px-1 font-bold">4/0AXxxxxxx</span>&scope=...
                    </p>
                    <p className="text-xs mt-1 text-red-600">
                      ↑ <strong>code=</strong> の後から <strong>&</strong> の前まで（黄色部分）をコピー
                    </p>
                  </div>

                  <div className="bg-yellow-50 border border-yellow-300 rounded p-2 text-xs text-yellow-800">
                    <strong>コピーの仕方：</strong> アドレスバーをクリック → 全部選択（Ctrl+A または Cmd+A）→ コピー（Ctrl+C または Cmd+C）→ 下の欄に貼り付けてから、code= より前と & より後ろを消す
                  </div>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="auth_code">認可コード（code=の後ろの値）</Label>
                  <Input
                    id="auth_code"
                    placeholder="4/0AX... で始まる文字列を貼り付け"
                    value={authCode}
                    onChange={(e) => setAuthCode(e.target.value)}
                  />
                  <p className="text-xs text-muted-foreground">
                    <code className="bg-muted px-1">4/</code> で始まる長い文字列です。<code className="bg-muted px-1">&scope</code> より後ろは含めないでください。
                  </p>
                </div>

                {exchangeError && (
                  <div className="bg-red-50 border border-red-200 rounded p-2 text-sm text-red-700">
                    {exchangeError}
                  </div>
                )}

                {exchangeSuccess && (
                  <div className="bg-green-50 border border-green-200 rounded p-2 text-sm text-green-700 flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4" />
                    認可に成功しました
                  </div>
                )}

                <Button
                  onClick={exchangeAuthCode}
                  disabled={exchanging || !authCode}
                >
                  {exchanging && <RefreshCw className="h-4 w-4 mr-2 animate-spin" />}
                  コードを交換
                </Button>
              </CardContent>
            </Card>
          </>
        )}
      </div>
    )
  }

  const renderStep4 = () => (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <Button onClick={fetchDevices} disabled={devicesLoading}>
          {devicesLoading && <RefreshCw className="h-4 w-4 mr-2 animate-spin" />}
          デバイスを取得
        </Button>
        <Badge variant="outline">{devices.length} 台</Badge>
      </div>

      {devicesError && (
        <div className="bg-red-50 border border-red-200 rounded p-3 text-sm text-red-700">
          {devicesError}
        </div>
      )}

      {devices.length === 0 && !devicesLoading && (
        <Card>
          <CardContent className="pt-4">
            <h4 className="font-medium text-sm mb-3">デバイスが見つからない場合のチェックリスト</h4>
            <ul className="space-y-2 text-sm">
              <li className="flex items-start gap-2">
                <input type="checkbox" className="mt-1" />
                <span>同じ Google アカウントで Home に Doorbell を登録していますか？</span>
              </li>
              <li className="flex items-start gap-2">
                <input type="checkbox" className="mt-1" />
                <span>Device Access Console で OAuth Client を紐付けましたか？</span>
              </li>
              <li className="flex items-start gap-2">
                <input type="checkbox" className="mt-1" />
                <span>認可URL でHome/デバイスへのアクセスを許可しましたか？</span>
              </li>
              <li className="flex items-start gap-2">
                <input type="checkbox" className="mt-1" />
                <span>複数 Home がある場合、正しい Home を選択しましたか？</span>
              </li>
              <li className="flex items-start gap-2">
                <input type="checkbox" className="mt-1" />
                <span>$5 課金を完了し、数分待ちましたか？</span>
              </li>
            </ul>
          </CardContent>
        </Card>
      )}

      {devices.length > 0 && (
        <Card>
          <CardContent className="pt-4">
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="text-left py-2 px-2">名前</th>
                    <th className="text-left py-2 px-2">タイプ</th>
                    <th className="text-left py-2 px-2">場所</th>
                    <th className="text-left py-2 px-2">機能</th>
                    <th className="py-2 px-2"></th>
                  </tr>
                </thead>
                <tbody>
                  {devices.map((device) => (
                    <tr key={device.sdm_device_id} className="border-b hover:bg-muted/50">
                      <td className="py-2 px-2 font-medium">{device.name || "Nest Doorbell"}</td>
                      <td className="py-2 px-2 text-muted-foreground">{device.device_type}</td>
                      <td className="py-2 px-2 text-muted-foreground">{device.room || "-"}</td>
                      <td className="py-2 px-2">
                        <div className="flex flex-wrap gap-1">
                          {device.traits_summary.slice(0, 3).map((trait) => (
                            <Badge key={trait} variant="outline" className="text-xs">
                              {trait}
                            </Badge>
                          ))}
                        </div>
                      </td>
                      <td className="py-2 px-2">
                        <Button
                          size="sm"
                          onClick={() => {
                            setSelectedDevice(device)
                            setCurrentStep(5)
                          }}
                        >
                          登録
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )

  const renderStep5 = () => (
    <div className="space-y-4">
      {selectedDevice && (
        <>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Bell className="h-4 w-4" />
                {selectedDevice.name || "Nest Doorbell"} を登録
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="camera_id">カメラID *</Label>
                <Input
                  id="camera_id"
                  placeholder="例: nest-abc123"
                  value={registerForm.camera_id}
                  onChange={(e) => setRegisterForm(prev => ({ ...prev, camera_id: e.target.value }))}
                />
                <p className="text-xs text-muted-foreground">
                  英小文字・数字・ハイフンのみ、最大64文字
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="name">表示名 *</Label>
                <Input
                  id="name"
                  placeholder="例: 玄関ドアベル"
                  value={registerForm.name}
                  onChange={(e) => setRegisterForm(prev => ({ ...prev, name: e.target.value }))}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="location">設置場所</Label>
                <Input
                  id="location"
                  placeholder="例: 1F 玄関"
                  value={registerForm.location}
                  onChange={(e) => setRegisterForm(prev => ({ ...prev, location: e.target.value }))}
                />
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="fid">施設ID (fid) *</Label>
                  <Input
                    id="fid"
                    placeholder="例: 0150"
                    value={registerForm.fid}
                    onChange={(e) => setRegisterForm(prev => ({ ...prev, fid: e.target.value }))}
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="tid">テナントID (tid) *</Label>
                  <Input
                    id="tid"
                    placeholder="例: T2025..."
                    value={registerForm.tid}
                    onChange={(e) => setRegisterForm(prev => ({ ...prev, tid: e.target.value }))}
                  />
                </div>
              </div>

              {registerError && (
                <div className="bg-red-50 border border-red-200 rounded p-2 text-sm text-red-700">
                  {registerError}
                </div>
              )}

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setSelectedDevice(null)
                    setCurrentStep(4)
                  }}
                >
                  キャンセル
                </Button>
                <Button
                  onClick={registerDevice}
                  disabled={registering || !registerForm.camera_id || !registerForm.name || !registerForm.fid || !registerForm.tid}
                >
                  {registering && <RefreshCw className="h-4 w-4 mr-2 animate-spin" />}
                  登録する
                </Button>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  )

  const renderStep6 = () => (
    <div className="space-y-4">
      {registeredCamera && (
        <>
          <div className="bg-green-50 border border-green-200 rounded-lg p-4">
            <div className="flex items-center gap-2 text-green-700">
              <CheckCircle2 className="h-5 w-5" />
              <span className="font-medium">登録完了</span>
            </div>
            <p className="text-sm text-green-600 mt-1">
              カメラ「{registeredCamera.name}」(ID: {registeredCamera.camera_id}) を登録しました。
            </p>
          </div>

          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Image className="h-4 w-4" />
                接続確認
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {snapshotLoading ? (
                <div className="flex items-center justify-center h-48 bg-muted rounded">
                  <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
                </div>
              ) : snapshotError ? (
                <div className="bg-red-50 border border-red-200 rounded p-4 text-center">
                  <XCircle className="h-8 w-8 text-red-500 mx-auto mb-2" />
                  <p className="text-sm text-red-700">{snapshotError}</p>
                  <Button variant="outline" size="sm" className="mt-2" onClick={fetchSnapshot}>
                    再試行
                  </Button>
                </div>
              ) : snapshotUrl ? (
                <div className="space-y-2">
                  <div className="relative bg-black rounded overflow-hidden">
                    <img
                      src={snapshotUrl}
                      alt="Doorbell snapshot"
                      className="w-full h-auto"
                      onError={() => setSnapshotError("画像の読み込みに失敗しました")}
                    />
                  </div>
                  <div className="flex items-center gap-2 text-green-600">
                    <CheckCircle2 className="h-4 w-4" />
                    <span className="text-sm">つながりました！</span>
                  </div>
                </div>
              ) : (
                <div className="flex items-center justify-center h-48 bg-muted rounded">
                  <Button onClick={fetchSnapshot}>
                    <Play className="h-4 w-4 mr-2" />
                    プレビューを取得
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>

          <div className="flex justify-center">
            <Button
              onClick={() => {
                // Reset wizard for another registration
                setCurrentStep(1)
                setSelectedDevice(null)
                setRegisteredCamera(null)
                setSnapshotUrl(null)
                setPrereqChecks({
                  hasProjectId: true,
                  hasClientId: true,
                  hasPaidFee: true,
                  hasDoorbellInHome: true,
                })
              }}
            >
              別のデバイスを登録
            </Button>
          </div>
        </>
      )}
    </div>
  )

  // =============================================================================
  // Render
  // =============================================================================

  return (
    <div className="flex h-full">
      {/* Main Content */}
      <div className="flex-1 flex flex-col">
        {/* Status Banner */}
        {config && (
          <div className="mb-4">
            {config.status === "error" && config.error_reason && (
              <div className="bg-red-50 border border-red-200 rounded-lg p-3 flex items-center gap-2">
                <XCircle className="h-5 w-5 text-red-500 flex-shrink-0" />
                <div>
                  <span className="text-sm font-medium text-red-700">エラー: </span>
                  <span className="text-sm text-red-600">{config.error_reason}</span>
                </div>
                <Button size="sm" variant="outline" className="ml-auto" onClick={() => setCurrentStep(3)}>
                  再認可
                </Button>
              </div>
            )}
            {config.status === "authorized" && (
              <div className="bg-green-50 border border-green-200 rounded-lg p-2 flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-green-500" />
                <span className="text-sm text-green-700">SDM連携が有効です</span>
                {config.last_synced_at && (
                  <span className="text-xs text-green-600 ml-auto">
                    最終同期: {new Date(config.last_synced_at).toLocaleString("ja-JP")}
                  </span>
                )}
              </div>
            )}
          </div>
        )}

        {/* Progress Indicator */}
        <div className="flex items-center justify-center gap-1 mb-4">
          {([1, 2, 3, 4, 5, 6] as WizardStep[]).map((step) => {
            const status = getStepStatus(step)
            const isCurrent = step === currentStep
            return (
              <button
                key={step}
                onClick={() => goToStep(step)}
                disabled={!status.accessible}
                className={`
                  flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors
                  ${isCurrent ? "bg-primary text-primary-foreground" : ""}
                  ${status.completed && !isCurrent ? "bg-green-100 text-green-700" : ""}
                  ${!status.accessible ? "opacity-50 cursor-not-allowed" : "hover:bg-muted cursor-pointer"}
                `}
              >
                {status.completed ? (
                  <Check className="h-3 w-3" />
                ) : (
                  <span className="w-3 text-center">{step}</span>
                )}
                <span className="hidden sm:inline">{STEP_TITLES[step]}</span>
              </button>
            )
          })}
        </div>

        {/* Step Content */}
        <ScrollArea className="flex-1">
          <div className="pr-4">
            <h3 className="text-lg font-semibold mb-4">
              Step {currentStep}: {STEP_TITLES[currentStep]}
            </h3>

            {configLoading ? (
              <div className="flex items-center justify-center py-8">
                <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : (
              <>
                {currentStep === 1 && renderStep1()}
                {currentStep === 2 && renderStep2()}
                {currentStep === 3 && renderStep3()}
                {currentStep === 4 && renderStep4()}
                {currentStep === 5 && renderStep5()}
                {currentStep === 6 && renderStep6()}
              </>
            )}
          </div>
        </ScrollArea>

        {/* Navigation */}
        <div className="flex items-center justify-between pt-4 border-t mt-4">
          <Button
            variant="outline"
            onClick={goBack}
            disabled={currentStep === 1}
          >
            <ChevronLeft className="h-4 w-4 mr-1" />
            戻る
          </Button>

          <span className="text-sm text-muted-foreground">
            {currentStep} / 6
          </span>

          <Button
            onClick={goNext}
            disabled={currentStep === 6 || !canProceed()}
          >
            次へ
            <ChevronRight className="h-4 w-4 ml-1" />
          </Button>
        </div>

        {/* Advanced Settings Toggle */}
        <div className="mt-4 pt-4 border-t">
          <button
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
          >
            {showAdvanced ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
            <Settings2 className="h-4 w-4" />
            高度な設定
          </button>

          {showAdvanced && (
            <div className="mt-3 space-y-3">
              {/* go2rtc Version Check */}
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={checkGo2rtcVersion} disabled={versionLoading}>
                  {versionLoading && <RefreshCw className="h-3 w-3 mr-1 animate-spin" />}
                  go2rtc バージョン確認
                </Button>
                {go2rtcVersion && (
                  <Badge variant={go2rtcVersion.compatible ? "default" : "destructive"}>
                    v{go2rtcVersion.version}
                    {go2rtcVersion.compatible ? " (対応)" : " (非対応)"}
                  </Badge>
                )}
              </div>

              {go2rtcVersion && !go2rtcVersion.compatible && (
                <div className="bg-red-50 border border-red-200 rounded p-2 text-xs text-red-700">
                  go2rtc v1.9.9 以上が必要です。バイナリを更新してください。
                </div>
              )}

              {/* Token Test */}
              <div className="flex items-center gap-2">
                <Button size="sm" variant="outline" onClick={testToken} disabled={tokenTesting || !config?.has_refresh_token}>
                  {tokenTesting && <RefreshCw className="h-3 w-3 mr-1 animate-spin" />}
                  トークンテスト
                </Button>
                {tokenTestResult && (
                  <Badge variant={tokenTestResult.ok ? "default" : "destructive"}>
                    {tokenTestResult.ok ? "OK" : tokenTestResult.error || "Failed"}
                  </Badge>
                )}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Help Panel */}
      <div className="w-64 ml-4 border-l pl-4 hidden lg:block">
        <button
          onClick={() => setShowHelp(!showHelp)}
          className="flex items-center gap-2 text-sm font-medium mb-3"
        >
          <HelpCircle className="h-4 w-4" />
          よくある詰まり
          {showHelp ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
        </button>

        {showHelp && (
          <div className="space-y-3 text-xs">
            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">課金していない/有効化待ち</p>
              <p className="text-muted-foreground">
                Device Access Console で $5 支払い後、数分待って再実行してください。
              </p>
            </div>

            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">Workspace アカウントで失敗</p>
              <p className="text-muted-foreground">
                個人の Gmail アカウントで試してください。
              </p>
            </div>

            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">Home にデバイスが見えない</p>
              <p className="text-muted-foreground">
                Google Home アプリで Doorbell が登録済みか確認してください。
              </p>
            </div>

            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">複数 Home がある</p>
              <p className="text-muted-foreground">
                認可URL を開いたときに正しい Home を選択してください。
              </p>
            </div>

            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">redirect_uri エラー</p>
              <p className="text-muted-foreground">
                OAuth Client の Redirect URI に{" "}
                <code className="bg-background px-1 rounded">https://www.google.com</code>{" "}
                が設定されているか確認してください。
              </p>
            </div>

            <div className="p-2 bg-muted rounded">
              <p className="font-medium mb-1">デバイスが 0 件</p>
              <p className="text-muted-foreground">
                Step 4 のチェックリストを順に確認し、再実行してください。
              </p>
            </div>
          </div>
        )}

        {!showHelp && (
          <p className="text-xs text-muted-foreground">
            クリックして表示
          </p>
        )}
      </div>
    </div>
  )
}
