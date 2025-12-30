#!/bin/bash
#
# IS21 camimageEdge AI インストールスクリプト
# Orange Pi 5 Plus (RK3588) 専用
#
# 使用方法:
#   sudo ./install.sh
#
# 前提条件:
#   - Orange Pi 5 Plus with RK3588
#   - Ubuntu 22.04 (Armbian)
#   - Python 3.10+
#   - rknn-toolkit2-lite (別途インストール)
#

set -e

# 色付きログ
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Root権限チェック
if [ "$EUID" -ne 0 ]; then
    log_error "root権限で実行してください: sudo ./install.sh"
    exit 1
fi

# スクリプトのディレクトリ取得
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_DIR="$(dirname "$SCRIPT_DIR")"

log_info "=== IS21 camimageEdge AI インストール ==="
log_info "ソースディレクトリ: $SRC_DIR"

# ディレクトリ作成
log_info "ディレクトリ作成..."
mkdir -p /opt/is21/src
mkdir -p /opt/is21/config
mkdir -p /opt/is21/models
mkdir -p /opt/is21/logs

# Pythonパッケージインストール
log_info "Pythonパッケージインストール..."
pip3 install --upgrade pip
pip3 install fastapi uvicorn[standard] pydantic numpy opencv-python-headless python-multipart

# ソースコードコピー
log_info "ソースコードコピー..."
cp -r "$SRC_DIR/src/"* /opt/is21/src/

# 設定ファイルコピー (存在する場合)
if [ -d "$SRC_DIR/config" ]; then
    log_info "設定ファイルコピー..."
    cp -r "$SRC_DIR/config/"* /opt/is21/config/ 2>/dev/null || true
fi

# systemdサービス設定
log_info "systemdサービス設定..."
cp "$SRC_DIR/systemd/is21-infer.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable is21-infer.service

# モデルファイル確認
log_info "モデルファイル確認..."
if [ ! -f "/opt/is21/models/yolov5s-640-640.rknn" ]; then
    log_warn "YOLOモデルが見つかりません: /opt/is21/models/yolov5s-640-640.rknn"
    log_warn "手動でモデルファイルを配置してください"
fi

if [ ! -f "/opt/is21/models/par_resnet50_pa100k.rknn" ]; then
    log_warn "PARモデルが見つかりません: /opt/is21/models/par_resnet50_pa100k.rknn"
    log_warn "手動でモデルファイルを配置してください"
fi

# パーミッション設定
log_info "パーミッション設定..."
chmod -R 755 /opt/is21

# サービス起動
log_info "サービス起動..."
systemctl start is21-infer.service
sleep 3

# 状態確認
if systemctl is-active --quiet is21-infer.service; then
    log_info "✓ IS21サービスが正常に起動しました"
    log_info "  API: http://$(hostname -I | awk '{print $1}'):9000/api/status"
else
    log_error "サービス起動に失敗しました"
    log_error "ログ確認: journalctl -u is21-infer.service -f"
    exit 1
fi

log_info ""
log_info "=== インストール完了 ==="
log_info ""
log_info "確認コマンド:"
log_info "  systemctl status is21-infer.service"
log_info "  curl http://localhost:9000/api/status"
log_info "  curl http://localhost:9000/healthz"
log_info ""
log_info "ログ確認:"
log_info "  journalctl -u is21-infer.service -f"
log_info ""
