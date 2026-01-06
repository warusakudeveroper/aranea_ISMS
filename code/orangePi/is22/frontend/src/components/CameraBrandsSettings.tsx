/**
 * CameraBrandsSettings - Camera Brand Management UI
 *
 * #84 T3-1 ~ T3-6: カメラブランド設定UI
 * - ブランド一覧・追加
 * - OUI管理
 * - RTSPテンプレート管理
 * - 汎用パス管理
 * - is_builtin編集制限表示
 */

import { useState, useEffect, useCallback } from "react"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion"
import {
  Plus,
  Trash2,
  Lock,
  Network,
  Video,
  Link,
  RefreshCw,
  ChevronRight,
} from "lucide-react"
import { API_BASE_URL } from "@/lib/config"

// Types
interface CameraBrand {
  id: number
  name: string
  display_name?: string
  category: string  // API returns "category" (consumer, professional, enterprise)
  is_builtin: boolean
  oui_count: number
  template_count: number
  created_at: string
}

interface OuiEntry {
  oui_prefix: string
  brand_id: number
  brand_name: string
  description: string | null
  status: "confirmed" | "candidate" | "investigating"
  is_builtin: boolean
}

interface RtspTemplate {
  id: number
  brand_id: number
  brand_name: string
  name: string
  main_path: string
  sub_path: string | null
  default_port: number
  requires_auth: boolean
  notes: string | null
  is_builtin: boolean
}

interface GenericRtspPath {
  id: number
  path: string
  description: string | null
  priority: number
  is_builtin: boolean
}

export function CameraBrandsSettings() {
  // Data states
  const [brands, setBrands] = useState<CameraBrand[]>([])
  const [ouiEntries, setOuiEntries] = useState<OuiEntry[]>([])
  const [templates, setTemplates] = useState<RtspTemplate[]>([])
  const [genericPaths, setGenericPaths] = useState<GenericRtspPath[]>([])
  const [loading, setLoading] = useState(false)

  // Modal states
  const [showAddBrand, setShowAddBrand] = useState(false)
  const [showAddOui, setShowAddOui] = useState(false)
  const [showAddTemplate, setShowAddTemplate] = useState(false)
  const [showAddGenericPath, setShowAddGenericPath] = useState(false)

  // Form states
  const [newBrand, setNewBrand] = useState({ name: "", display_name: "", category: "consumer" })
  const [newOui, setNewOui] = useState<{ oui_prefix: string; brand_id: number; description: string; status: "confirmed" | "candidate" | "investigating" }>({ oui_prefix: "", brand_id: 0, description: "", status: "candidate" })
  const [newTemplate, setNewTemplate] = useState({
    brand_id: 0,
    name: "",
    main_path: "/stream1",
    sub_path: "/stream2",
    default_port: 554,
    requires_auth: true,
    notes: ""
  })
  const [newGenericPath, setNewGenericPath] = useState({ path: "", description: "", priority: 100 })

  // Selected brand for OUI/Template context (used to pre-select in modals)
  const [, setSelectedBrandId] = useState<number | null>(null)

  // Fetch all data
  const fetchData = useCallback(async () => {
    setLoading(true)
    try {
      const [brandsRes, ouiRes, templatesRes, pathsRes] = await Promise.all([
        fetch(`${API_BASE_URL}/api/settings/camera-brands`),
        fetch(`${API_BASE_URL}/api/settings/oui`),
        fetch(`${API_BASE_URL}/api/settings/rtsp-templates`),
        fetch(`${API_BASE_URL}/api/settings/generic-rtsp-paths`),
      ])

      if (brandsRes.ok) {
        const json = await brandsRes.json()
        // API returns {ok: true, data: [...]} format
        setBrands(Array.isArray(json.data) ? json.data : (json.data?.brands || json.brands || []))
      }
      if (ouiRes.ok) {
        const json = await ouiRes.json()
        setOuiEntries(Array.isArray(json.data) ? json.data : (json.data?.entries || json.entries || []))
      }
      if (templatesRes.ok) {
        const json = await templatesRes.json()
        setTemplates(Array.isArray(json.data) ? json.data : (json.data?.templates || json.templates || []))
      }
      if (pathsRes.ok) {
        const json = await pathsRes.json()
        setGenericPaths(Array.isArray(json.data) ? json.data : (json.data?.paths || json.paths || []))
      }
    } catch (error) {
      console.error("Failed to fetch camera brand data:", error)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  // Add Brand
  const handleAddBrand = async () => {
    if (!newBrand.name.trim()) return
    try {
      const brandData = {
        name: newBrand.name,
        display_name: newBrand.display_name || newBrand.name, // Use name if display_name not set
        category: newBrand.category,
      }
      const resp = await fetch(`${API_BASE_URL}/api/settings/camera-brands`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(brandData),
      })
      if (resp.ok) {
        setShowAddBrand(false)
        setNewBrand({ name: "", display_name: "", category: "consumer" })
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || json.message || "追加に失敗しました")
      }
    } catch (error) {
      console.error("Failed to add brand:", error)
    }
  }

  // Delete Brand (only non-builtin)
  const handleDeleteBrand = async (id: number) => {
    if (!confirm("このブランドを削除しますか？関連するOUIとテンプレートも削除されます。")) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/camera-brands/${id}`, {
        method: "DELETE",
      })
      if (resp.ok) {
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "削除に失敗しました")
      }
    } catch (error) {
      console.error("Failed to delete brand:", error)
    }
  }

  // Add OUI
  const handleAddOui = async () => {
    if (!newOui.oui_prefix.trim() || !newOui.brand_id) return
    // Validate OUI format (XX:XX:XX)
    const ouiRegex = /^[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}:[0-9A-Fa-f]{2}$/
    if (!ouiRegex.test(newOui.oui_prefix)) {
      alert("OUIは XX:XX:XX 形式で入力してください（例: 00:1A:2B）")
      return
    }
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/camera-brands/${newOui.brand_id}/oui`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          oui_prefix: newOui.oui_prefix.toUpperCase(),
          description: newOui.description || null,
          status: newOui.status,
        }),
      })
      if (resp.ok) {
        setShowAddOui(false)
        setNewOui({ oui_prefix: "", brand_id: 0, description: "", status: "candidate" })
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "追加に失敗しました")
      }
    } catch (error) {
      console.error("Failed to add OUI:", error)
    }
  }

  // Delete OUI (only non-builtin)
  const handleDeleteOui = async (prefix: string) => {
    if (!confirm(`OUI ${prefix} を削除しますか？`)) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/oui/${encodeURIComponent(prefix)}`, {
        method: "DELETE",
      })
      if (resp.ok) {
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "削除に失敗しました")
      }
    } catch (error) {
      console.error("Failed to delete OUI:", error)
    }
  }

  // Add Template
  const handleAddTemplate = async () => {
    if (!newTemplate.name.trim() || !newTemplate.brand_id || !newTemplate.main_path.trim()) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/camera-brands/${newTemplate.brand_id}/rtsp-templates`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: newTemplate.name,
          main_path: newTemplate.main_path,
          sub_path: newTemplate.sub_path || null,
          default_port: newTemplate.default_port,
          requires_auth: newTemplate.requires_auth,
          notes: newTemplate.notes || null,
        }),
      })
      if (resp.ok) {
        setShowAddTemplate(false)
        setNewTemplate({
          brand_id: 0,
          name: "",
          main_path: "/stream1",
          sub_path: "/stream2",
          default_port: 554,
          requires_auth: true,
          notes: ""
        })
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "追加に失敗しました")
      }
    } catch (error) {
      console.error("Failed to add template:", error)
    }
  }

  // Delete Template (only non-builtin)
  const handleDeleteTemplate = async (id: number) => {
    if (!confirm("このテンプレートを削除しますか？")) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/rtsp-templates/${id}`, {
        method: "DELETE",
      })
      if (resp.ok) {
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "削除に失敗しました")
      }
    } catch (error) {
      console.error("Failed to delete template:", error)
    }
  }

  // Add Generic Path
  const handleAddGenericPath = async () => {
    if (!newGenericPath.path.trim()) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/generic-rtsp-paths`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(newGenericPath),
      })
      if (resp.ok) {
        setShowAddGenericPath(false)
        setNewGenericPath({ path: "", description: "", priority: 100 })
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "追加に失敗しました")
      }
    } catch (error) {
      console.error("Failed to add generic path:", error)
    }
  }

  // Delete Generic Path (only non-builtin)
  const handleDeleteGenericPath = async (id: number) => {
    if (!confirm("この汎用パスを削除しますか？")) return
    try {
      const resp = await fetch(`${API_BASE_URL}/api/settings/generic-rtsp-paths/${id}`, {
        method: "DELETE",
      })
      if (resp.ok) {
        fetchData()
      } else {
        const json = await resp.json()
        alert(json.error || "削除に失敗しました")
      }
    } catch (error) {
      console.error("Failed to delete generic path:", error)
    }
  }

  // Get status badge color
  const getStatusBadge = (status: string) => {
    switch (status) {
      case "confirmed":
        return <Badge className="bg-green-500">確認済</Badge>
      case "candidate":
        return <Badge className="bg-yellow-500">候補</Badge>
      case "investigating":
        return <Badge className="bg-blue-500">調査中</Badge>
      default:
        return <Badge variant="outline">{status}</Badge>
    }
  }

  return (
    <div className="space-y-4">
      {/* Header with refresh */}
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-medium flex items-center gap-2">
          <Video className="h-5 w-5" />
          カメラブランド設定
        </h3>
        <Button variant="outline" size="sm" onClick={fetchData} disabled={loading}>
          {loading ? <RefreshCw className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
        </Button>
      </div>

      <Accordion type="multiple" defaultValue={["brands", "generic"]} className="space-y-2">
        {/* Brands Section */}
        <AccordionItem value="brands" className="border rounded-lg">
          <AccordionTrigger className="px-4 hover:no-underline">
            <div className="flex items-center gap-2">
              <Video className="h-4 w-4" />
              <span>カメラブランド</span>
              <Badge variant="outline">{brands.length}</Badge>
            </div>
          </AccordionTrigger>
          <AccordionContent className="px-4 pb-4">
            <div className="space-y-3">
              <Button size="sm" onClick={() => setShowAddBrand(true)}>
                <Plus className="h-4 w-4 mr-1" />
                ブランド追加
              </Button>

              <ScrollArea className="h-64">
                <div className="space-y-2">
                  {brands.map((brand) => (
                    <Card key={brand.id} className="p-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          {brand.is_builtin && <span title="組込（編集不可）"><Lock className="h-4 w-4 text-muted-foreground" /></span>}
                          <div>
                            <p className="font-medium">{brand.name}</p>
                            <p className="text-xs text-muted-foreground">
                              {brand.category} | OUI: {brand.oui_count} | テンプレート: {brand.template_count}
                            </p>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => {
                              setSelectedBrandId(brand.id)
                              setNewOui({ ...newOui, brand_id: brand.id })
                              setShowAddOui(true)
                            }}
                            title="OUI追加"
                          >
                            <Network className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => {
                              setSelectedBrandId(brand.id)
                              setNewTemplate({ ...newTemplate, brand_id: brand.id })
                              setShowAddTemplate(true)
                            }}
                            title="テンプレート追加"
                          >
                            <Link className="h-4 w-4" />
                          </Button>
                          {!brand.is_builtin && (
                            <Button
                              variant="ghost"
                              size="icon"
                              onClick={() => handleDeleteBrand(brand.id)}
                              className="text-destructive"
                            >
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          )}
                        </div>
                      </div>
                    </Card>
                  ))}
                </div>
              </ScrollArea>
            </div>
          </AccordionContent>
        </AccordionItem>

        {/* OUI Section */}
        <AccordionItem value="oui" className="border rounded-lg">
          <AccordionTrigger className="px-4 hover:no-underline">
            <div className="flex items-center gap-2">
              <Network className="h-4 w-4" />
              <span>OUIエントリ</span>
              <Badge variant="outline">{ouiEntries.length}</Badge>
            </div>
          </AccordionTrigger>
          <AccordionContent className="px-4 pb-4">
            <ScrollArea className="h-64">
              <div className="space-y-2">
                {ouiEntries.map((oui) => (
                  <Card key={oui.oui_prefix} className="p-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        {oui.is_builtin && <Lock className="h-4 w-4 text-muted-foreground" />}
                        <div>
                          <div className="flex items-center gap-2">
                            <p className="font-mono font-medium">{oui.oui_prefix}</p>
                            {getStatusBadge(oui.status)}
                          </div>
                          <p className="text-xs text-muted-foreground">
                            {oui.brand_name} {oui.description && `- ${oui.description}`}
                          </p>
                        </div>
                      </div>
                      {!oui.is_builtin && (
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => handleDeleteOui(oui.oui_prefix)}
                          className="text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </AccordionContent>
        </AccordionItem>

        {/* Templates Section */}
        <AccordionItem value="templates" className="border rounded-lg">
          <AccordionTrigger className="px-4 hover:no-underline">
            <div className="flex items-center gap-2">
              <Link className="h-4 w-4" />
              <span>RTSPテンプレート</span>
              <Badge variant="outline">{templates.length}</Badge>
            </div>
          </AccordionTrigger>
          <AccordionContent className="px-4 pb-4">
            <ScrollArea className="h-64">
              <div className="space-y-2">
                {templates.map((tmpl) => (
                  <Card key={tmpl.id} className="p-3">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        {tmpl.is_builtin && <Lock className="h-4 w-4 text-muted-foreground" />}
                        <div>
                          <p className="font-medium">{tmpl.name}</p>
                          <p className="text-xs font-mono text-muted-foreground">
                            Main: {tmpl.main_path} | Port: {tmpl.default_port}
                          </p>
                          {tmpl.sub_path && (
                            <p className="text-xs font-mono text-muted-foreground">
                              Sub: {tmpl.sub_path}
                            </p>
                          )}
                          <p className="text-xs text-muted-foreground">
                            {tmpl.brand_name} {tmpl.requires_auth && "| 認証必須"}
                          </p>
                        </div>
                      </div>
                      {!tmpl.is_builtin && (
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={() => handleDeleteTemplate(tmpl.id)}
                          className="text-destructive"
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                  </Card>
                ))}
              </div>
            </ScrollArea>
          </AccordionContent>
        </AccordionItem>

        {/* Generic Paths Section */}
        <AccordionItem value="generic" className="border rounded-lg">
          <AccordionTrigger className="px-4 hover:no-underline">
            <div className="flex items-center gap-2">
              <ChevronRight className="h-4 w-4" />
              <span>汎用RTSPパス</span>
              <Badge variant="outline">{genericPaths.length}</Badge>
            </div>
          </AccordionTrigger>
          <AccordionContent className="px-4 pb-4">
            <div className="space-y-3">
              <Button size="sm" onClick={() => setShowAddGenericPath(true)}>
                <Plus className="h-4 w-4 mr-1" />
                パス追加
              </Button>

              <ScrollArea className="h-48">
                <div className="space-y-2">
                  {genericPaths.sort((a, b) => a.priority - b.priority).map((path) => (
                    <Card key={path.id} className="p-3">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          {path.is_builtin && <Lock className="h-4 w-4 text-muted-foreground" />}
                          <div>
                            <div className="flex items-center gap-2">
                              <p className="font-mono font-medium">{path.path}</p>
                              <Badge variant="outline" className="text-xs">優先度: {path.priority}</Badge>
                            </div>
                            {path.description && (
                              <p className="text-xs text-muted-foreground">{path.description}</p>
                            )}
                          </div>
                        </div>
                        {!path.is_builtin && (
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => handleDeleteGenericPath(path.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    </Card>
                  ))}
                </div>
              </ScrollArea>
            </div>
          </AccordionContent>
        </AccordionItem>
      </Accordion>

      {/* Add Brand Modal */}
      <Dialog open={showAddBrand} onOpenChange={setShowAddBrand}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>ブランド追加</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>ブランド名（識別子）</Label>
              <Input
                placeholder="例: Hikvision"
                value={newBrand.name}
                onChange={(e) => setNewBrand({ ...newBrand, name: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">内部識別用（英数字推奨）</p>
            </div>
            <div className="space-y-2">
              <Label>表示名</Label>
              <Input
                placeholder="例: Hikvision（海康威視）"
                value={newBrand.display_name}
                onChange={(e) => setNewBrand({ ...newBrand, display_name: e.target.value })}
              />
              <p className="text-xs text-muted-foreground">空欄の場合はブランド名を使用</p>
            </div>
            <div className="space-y-2">
              <Label>カテゴリ</Label>
              <Select value={newBrand.category} onValueChange={(v: string) => setNewBrand({ ...newBrand, category: v })}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="consumer">Consumer（一般向け）</SelectItem>
                  <SelectItem value="professional">Professional（業務用）</SelectItem>
                  <SelectItem value="enterprise">Enterprise（企業向け）</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddBrand(false)}>キャンセル</Button>
            <Button onClick={handleAddBrand}>追加</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add OUI Modal */}
      <Dialog open={showAddOui} onOpenChange={setShowAddOui}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>OUI追加</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>ブランド</Label>
              <Select
                value={String(newOui.brand_id || "")}
                onValueChange={(v: string) => setNewOui({ ...newOui, brand_id: parseInt(v) })}
              >
                <SelectTrigger>
                  <SelectValue placeholder="ブランドを選択" />
                </SelectTrigger>
                <SelectContent>
                  {brands.map((b) => (
                    <SelectItem key={b.id} value={String(b.id)}>{b.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>OUIプレフィックス</Label>
              <Input
                placeholder="XX:XX:XX"
                value={newOui.oui_prefix}
                onChange={(e) => setNewOui({ ...newOui, oui_prefix: e.target.value.toUpperCase() })}
              />
              <p className="text-xs text-muted-foreground">MACアドレスの先頭3バイト（例: 00:1A:2B）</p>
            </div>
            <div className="space-y-2">
              <Label>ステータス</Label>
              <Select value={newOui.status} onValueChange={(v: string) => setNewOui({ ...newOui, status: v as "confirmed" | "candidate" | "investigating" })}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="confirmed">確認済</SelectItem>
                  <SelectItem value="candidate">候補</SelectItem>
                  <SelectItem value="investigating">調査中</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>説明（任意）</Label>
              <Input
                placeholder="カメラシリーズ名など"
                value={newOui.description}
                onChange={(e) => setNewOui({ ...newOui, description: e.target.value })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddOui(false)}>キャンセル</Button>
            <Button onClick={handleAddOui}>追加</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Template Modal */}
      <Dialog open={showAddTemplate} onOpenChange={setShowAddTemplate}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>RTSPテンプレート追加</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>ブランド</Label>
              <Select
                value={String(newTemplate.brand_id || "")}
                onValueChange={(v: string) => setNewTemplate({ ...newTemplate, brand_id: parseInt(v) })}
              >
                <SelectTrigger>
                  <SelectValue placeholder="ブランドを選択" />
                </SelectTrigger>
                <SelectContent>
                  {brands.map((b) => (
                    <SelectItem key={b.id} value={String(b.id)}>{b.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>テンプレート名</Label>
              <Input
                placeholder="例: Default HD"
                value={newTemplate.name}
                onChange={(e) => setNewTemplate({ ...newTemplate, name: e.target.value })}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>メインパス</Label>
                <Input
                  placeholder="/stream1"
                  value={newTemplate.main_path}
                  onChange={(e) => setNewTemplate({ ...newTemplate, main_path: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label>サブパス</Label>
                <Input
                  placeholder="/stream2"
                  value={newTemplate.sub_path}
                  onChange={(e) => setNewTemplate({ ...newTemplate, sub_path: e.target.value })}
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>デフォルトポート</Label>
              <Input
                type="number"
                value={newTemplate.default_port}
                onChange={(e) => setNewTemplate({ ...newTemplate, default_port: parseInt(e.target.value) || 554 })}
              />
            </div>
            <div className="space-y-2">
              <Label>メモ（任意）</Label>
              <Input
                placeholder="追加情報"
                value={newTemplate.notes}
                onChange={(e) => setNewTemplate({ ...newTemplate, notes: e.target.value })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddTemplate(false)}>キャンセル</Button>
            <Button onClick={handleAddTemplate}>追加</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Generic Path Modal */}
      <Dialog open={showAddGenericPath} onOpenChange={setShowAddGenericPath}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>汎用パス追加</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>RTSPパス</Label>
              <Input
                placeholder="/stream1"
                value={newGenericPath.path}
                onChange={(e) => setNewGenericPath({ ...newGenericPath, path: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label>優先度</Label>
              <Input
                type="number"
                min={1}
                max={1000}
                value={newGenericPath.priority}
                onChange={(e) => setNewGenericPath({ ...newGenericPath, priority: parseInt(e.target.value) || 100 })}
              />
              <p className="text-xs text-muted-foreground">小さいほど優先的に試行（1-1000）</p>
            </div>
            <div className="space-y-2">
              <Label>説明（任意）</Label>
              <Input
                placeholder="一般的なパス"
                value={newGenericPath.description}
                onChange={(e) => setNewGenericPath({ ...newGenericPath, description: e.target.value })}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddGenericPath(false)}>キャンセル</Button>
            <Button onClick={handleAddGenericPath}>追加</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
