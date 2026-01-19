#!/bin/bash
#
# is20s Update Script - Deploy app updates to remote servers
# Usage: bash update.sh [target]
#   target: tam, toj, akb, all (default: all)
#
# Requires: sshpass (brew install hudochenkov/sshpass/sshpass)
#

# Don't exit on error - we handle failures per-target
# set -e

# Server configuration
TAM_IP="192.168.125.248"
TOJ_IP="192.168.126.249"
AKB_IP="192.168.3.250"

SSH_USER="mijeosadmin"
SSH_PASS="mijeos12345@"
REMOTE_APP_DIR="/opt/is20s/app"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCAL_APP_DIR="$SCRIPT_DIR/../app"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${CYAN}[STEP]${NC} $1"; }

get_ip() {
    case "$1" in
        tam) echo "$TAM_IP" ;;
        toj) echo "$TOJ_IP" ;;
        akb) echo "$AKB_IP" ;;
        *) echo "" ;;
    esac
}

deploy_to() {
    local name="$1"
    local IP=$(get_ip "$name")

    if [ -z "$IP" ]; then
        log_error "Unknown target: $name"
        return 1
    fi

    log_step "[$name] Deploying to $IP..."

    # Test SSH connection
    if ! sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$SSH_USER@$IP" "echo ok" &>/dev/null; then
        log_error "[$name] SSH connection failed"
        return 1
    fi

    # Rsync files
    log_info "[$name] Syncing files..."
    if ! sshpass -p "$SSH_PASS" rsync -avz --delete \
        -e "ssh -o StrictHostKeyChecking=no" \
        "$LOCAL_APP_DIR/" "$SSH_USER@$IP:$REMOTE_APP_DIR/" 2>&1 | tail -3; then
        log_error "[$name] File sync failed"
        return 1
    fi

    # Restart service
    log_info "[$name] Restarting service..."
    sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no "$SSH_USER@$IP" \
        "echo '$SSH_PASS' | sudo -S systemctl restart is20s" &>/dev/null || true

    sleep 2

    # Check service status
    local STATUS=$(sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no "$SSH_USER@$IP" \
        "systemctl is-active is20s" 2>/dev/null || echo "unknown")

    if [ "$STATUS" = "active" ]; then
        log_info "[$name] Service: ${GREEN}active${NC}"
        return 0
    else
        log_error "[$name] Service: $STATUS"
        return 1
    fi
}

# Check sshpass
if ! command -v sshpass &>/dev/null; then
    log_error "sshpass is required. Install with: brew install hudochenkov/sshpass/sshpass"
    exit 1
fi

# Parse target
TARGET="${1:-all}"

log_info "=========================================="
log_info "  is20s Update Script"
log_info "=========================================="
log_info "Source:  $LOCAL_APP_DIR"
log_info "Target:  $TARGET"
log_info ""

# Check local files
log_step "Checking local files..."
if [ ! -d "$LOCAL_APP_DIR" ]; then
    log_error "Local app directory not found: $LOCAL_APP_DIR"
    exit 1
fi
FILE_COUNT=$(find "$LOCAL_APP_DIR" -name "*.py" | wc -l | tr -d ' ')
log_info "Found $FILE_COUNT Python files"

# Deploy
SUCCESS_COUNT=0
FAIL_COUNT=0

if [ "$TARGET" = "all" ]; then
    TARGETS="tam toj akb"
else
    TARGETS="$TARGET"
fi

for name in $TARGETS; do
    if deploy_to "$name"; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
done

# Summary
log_info "=========================================="
log_info "  Deployment Summary"
log_info "=========================================="
log_info "Success: $SUCCESS_COUNT"
if [ $FAIL_COUNT -gt 0 ]; then
    log_error "Failed:  $FAIL_COUNT"
    exit 1
fi
log_info "All deployments completed successfully!"
