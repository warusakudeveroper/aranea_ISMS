import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { AlertTriangle, Info, CheckCircle } from "lucide-react"
import { cn } from "@/lib/utils"

type DialogType = "info" | "warning" | "danger" | "success"

interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  confirmLabel?: string
  cancelLabel?: string
  type?: DialogType
  onConfirm: () => void
  onCancel?: () => void
  loading?: boolean
}

const typeStyles: Record<
  DialogType,
  { icon: typeof Info; iconClass: string; buttonClass: string }
> = {
  info: {
    icon: Info,
    iconClass: "text-blue-500",
    buttonClass: "bg-blue-600 hover:bg-blue-700",
  },
  warning: {
    icon: AlertTriangle,
    iconClass: "text-amber-500",
    buttonClass: "bg-amber-600 hover:bg-amber-700",
  },
  danger: {
    icon: AlertTriangle,
    iconClass: "text-red-500",
    buttonClass: "bg-red-600 hover:bg-red-700",
  },
  success: {
    icon: CheckCircle,
    iconClass: "text-green-500",
    buttonClass: "bg-green-600 hover:bg-green-700",
  },
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = "確認",
  cancelLabel = "キャンセル",
  type = "info",
  onConfirm,
  onCancel,
  loading = false,
}: ConfirmDialogProps) {
  const style = typeStyles[type]
  const Icon = style.icon

  const handleCancel = () => {
    onCancel?.()
    onOpenChange(false)
  }

  const handleConfirm = () => {
    onConfirm()
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <div className="flex items-start gap-3">
            <div
              className={cn(
                "flex h-10 w-10 items-center justify-center rounded-full",
                type === "danger" && "bg-red-100 dark:bg-red-900/30",
                type === "warning" && "bg-amber-100 dark:bg-amber-900/30",
                type === "info" && "bg-blue-100 dark:bg-blue-900/30",
                type === "success" && "bg-green-100 dark:bg-green-900/30"
              )}
            >
              <Icon className={cn("h-5 w-5", style.iconClass)} />
            </div>
            <div className="flex-1">
              <DialogTitle>{title}</DialogTitle>
              <DialogDescription className="mt-2">
                {description}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>
        <DialogFooter className="mt-4 gap-2">
          <Button variant="outline" onClick={handleCancel} disabled={loading}>
            {cancelLabel}
          </Button>
          <Button
            className={cn("text-white", style.buttonClass)}
            onClick={handleConfirm}
            disabled={loading}
          >
            {loading ? "処理中..." : confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
