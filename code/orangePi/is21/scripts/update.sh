#!/bin/bash
#
# IS21 ソースコード更新スクリプト
# サービス再起動を含む
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

if [ "$EUID" -ne 0 ]; then
    log_error "root権限で実行してください: sudo ./update.sh"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC_DIR="$(dirname "$SCRIPT_DIR")"

log_info "=== IS21 ソースコード更新 ==="

# サービス停止
log_info "サービス停止..."
systemctl stop is21-infer.service || true

# ソースコードコピー
log_info "ソースコード更新..."
cp -r "$SRC_DIR/src/"* /opt/is21/src/

# サービス再起動
log_info "サービス再起動..."
systemctl start is21-infer.service
sleep 3

# 状態確認
if systemctl is-active --quiet is21-infer.service; then
    log_info "✓ 更新完了"

    # バージョン確認
    VERSION=$(curl -s http://localhost:9000/api/status 2>/dev/null | grep -o '"firmwareVersion":"[^"]*"' | cut -d'"' -f4)
    log_info "  現在のバージョン: $VERSION"
else
    log_error "サービス起動に失敗しました"
    journalctl -u is21-infer.service -n 20 --no-pager
    exit 1
fi
