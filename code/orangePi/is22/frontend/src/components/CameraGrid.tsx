import type { Camera } from "@/types/api"
import { CameraTile } from "./CameraTile"
import { cn } from "@/lib/utils"
import { API_BASE_URL } from "@/lib/config"
import { Card } from "@/components/ui/card"
import { Search, PlusCircle } from "lucide-react"

interface CameraGridProps {
  cameras: Camera[]
  onCameraClick?: (camera: Camera) => void
  onSettingsClick?: (camera: Camera) => void
  onAddClick?: () => void
}

function getGridClass(count: number): string {
  // 基本3列、カメラ数に応じてスケール（末尾の追加カードも含む）
  if (count <= 3) return "grid-cols-3"
  if (count <= 9) return "grid-cols-3"
  if (count <= 16) return "grid-cols-4"
  if (count <= 25) return "grid-cols-5"
  return "grid-cols-6"
}

// 末尾追加カード（BTN-003相当）
function AddCameraCard({ onClick }: { onClick?: () => void }) {
  return (
    <Card
      className="aspect-video cursor-pointer border-2 border-dashed border-muted-foreground/30 hover:border-primary/50 hover:bg-accent/30 transition-all flex flex-col items-center justify-center gap-2"
      onClick={onClick}
    >
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted">
        <PlusCircle className="h-5 w-5 text-muted-foreground" />
      </div>
      <div className="flex items-center gap-1 text-sm text-muted-foreground">
        <Search className="h-4 w-4" />
        <span>カメラをスキャン</span>
      </div>
    </Card>
  )
}

export function CameraGrid({ cameras, onCameraClick, onSettingsClick, onAddClick }: CameraGridProps) {
  // 末尾カードも含めたカウントでグリッド計算
  const gridClass = getGridClass(cameras.length + 1)

  return (
    <div className={cn("grid gap-2", gridClass)}>
      {cameras.map((camera) => (
        <CameraTile
          key={camera.camera_id}
          camera={camera}
          snapshotUrl={`${API_BASE_URL}/api/streams/${camera.camera_id}/snapshot`}
          onClick={() => onCameraClick?.(camera)}
          onSettingsClick={() => onSettingsClick?.(camera)}
        />
      ))}
      {/* 末尾に追加カード (IS22_UI_DETAILED_SPEC Section 2.2 追加要件) */}
      <AddCameraCard onClick={onAddClick} />
    </div>
  )
}
