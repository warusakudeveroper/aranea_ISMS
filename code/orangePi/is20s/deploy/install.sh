#!/bin/bash
#
# is20s Installer for Orange Pi Zero3 (Armbian)
# Usage: sudo bash install.sh
#
# Required files in same directory:
#   - is20s.tar.gz    (application)
#   - global.tar.gz   (shared library)
#   - is20s.service   (systemd unit)
#

set -e

# Configuration
APP_USER="mijeosadmin"
APP_DIR="/opt/is20s"
GLOBAL_DIR="/opt/global"
DATA_DIR="/var/lib/is20s"
LOG_DIR="/var/log/is20s"
SERVICE_NAME="is20s"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check root
if [ "$EUID" -ne 0 ]; then
    log_error "Please run as root (sudo bash install.sh)"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

log_info "=========================================="
log_info "  is20s Installer"
log_info "=========================================="
log_info "App directory:    $APP_DIR"
log_info "Global directory: $GLOBAL_DIR"
log_info "Data directory:   $DATA_DIR"
log_info "Log directory:    $LOG_DIR"
log_info "User:             $APP_USER"
log_info ""

# Check required files
log_info "[1/10] Checking required files..."
for f in is20s.tar.gz global.tar.gz is20s.service; do
    if [ ! -f "$SCRIPT_DIR/$f" ]; then
        log_error "Required file not found: $f"
        exit 1
    fi
done
log_info "All required files present"

# Check user exists
log_info "[2/10] Checking user..."
if ! id "$APP_USER" &>/dev/null; then
    log_error "User '$APP_USER' does not exist. Create user first."
    exit 1
fi
log_info "User $APP_USER exists"

# Install system dependencies
log_info "[3/10] Installing system dependencies..."
apt-get update -qq

# Preconfigure wireshark to allow non-root capture (avoid interactive dialog)
echo "wireshark-common wireshark-common/install-setuid boolean true" | debconf-set-selections

DEBIAN_FRONTEND=noninteractive apt-get install -y -qq python3-venv python3-pip tshark sqlite3
log_info "System packages installed"

# Configure tshark for non-root capture
log_info "[4/10] Configuring tshark permissions..."
if ! getent group wireshark > /dev/null 2>&1; then
    groupadd wireshark
fi
usermod -aG wireshark "$APP_USER"
chgrp wireshark /usr/bin/dumpcap 2>/dev/null || true
chmod 750 /usr/bin/dumpcap 2>/dev/null || true
if command -v setcap &>/dev/null; then
    setcap cap_net_raw,cap_net_admin=eip /usr/bin/dumpcap 2>/dev/null || true
fi
log_info "tshark configured for non-root capture"

# Create directories
log_info "[5/10] Creating directories..."
mkdir -p "$APP_DIR"
mkdir -p "$GLOBAL_DIR"
mkdir -p "$DATA_DIR"
mkdir -p "$DATA_DIR/capture_logs"
mkdir -p "$LOG_DIR"
log_info "Directories created"

# Extract application
log_info "[6/10] Extracting application files..."
tar -xzf "$SCRIPT_DIR/is20s.tar.gz" -C "$APP_DIR"
tar -xzf "$SCRIPT_DIR/global.tar.gz" -C /opt/
log_info "Application extracted"

# Set ownership
log_info "[7/10] Setting permissions..."
chown -R "$APP_USER:$APP_USER" "$APP_DIR"
chown -R "$APP_USER:$APP_USER" "$GLOBAL_DIR"
chown -R "$APP_USER:$APP_USER" "$DATA_DIR"
chown -R "$APP_USER:$APP_USER" "$LOG_DIR"
chmod -R 755 "$APP_DIR"
chmod -R 755 "$GLOBAL_DIR"
chmod 777 "$DATA_DIR/capture_logs"
log_info "Permissions set"

# Configure polkit for NetworkManager (WiFi control)
log_info "[8/11] Configuring polkit for WiFi control..."
mkdir -p /etc/polkit-1/rules.d
cat > /etc/polkit-1/rules.d/10-network-manager.rules << 'EOF'
polkit.addRule(function(action, subject) {
    if (action.id.indexOf("org.freedesktop.NetworkManager") === 0 &&
        subject.user === "mijeosadmin") {
        return polkit.Result.YES;
    }
});
EOF
chmod 644 /etc/polkit-1/rules.d/10-network-manager.rules
systemctl restart polkit 2>/dev/null || true
log_info "polkit configured for NetworkManager access"

# Create Python virtual environment
log_info "[9/11] Creating Python virtual environment..."
cd "$APP_DIR"
sudo -u "$APP_USER" python3 -m venv .venv
sudo -u "$APP_USER" "$APP_DIR/.venv/bin/pip" install --upgrade pip -q
sudo -u "$APP_USER" "$APP_DIR/.venv/bin/pip" install -r "$APP_DIR/requirements.txt" -q
log_info "Python environment ready"

# Create default config files
log_info "[10/11] Creating default configurations..."
if [ ! -f "$DATA_DIR/watch_config.json" ]; then
    cat > "$DATA_DIR/watch_config.json" << 'EOF'
{
  "timezone": "Asia/Tokyo",
  "rooms": {},
  "capture": {
    "iface": "end0",
    "snaplen": 2048,
    "buffer_mb": 16,
    "ports": [53, 80, 443],
    "include_quic_udp_443": true,
    "include_tcp_syn_any_port": true,
    "syn_only": true
  },
  "post": {
    "url": "",
    "headers": {},
    "batch_max_events": 500,
    "batch_max_seconds": 30,
    "gzip": true,
    "max_queue_events": 20000
  },
  "mode": {
    "dry_run": true,
    "enabled": true
  },
  "logging": {
    "enabled": true,
    "log_dir": "/var/lib/is20s/capture_logs",
    "max_file_size": 1048576,
    "max_total_size": 10485760
  }
}
EOF
    chown "$APP_USER:$APP_USER" "$DATA_DIR/watch_config.json"
fi

# Create default WiFi configs
if [ ! -f "$DATA_DIR/wifi_configs.json" ]; then
    cat > "$DATA_DIR/wifi_configs.json" << 'EOF'
[
  {"ssid": "cluster1", "password": "isms12345@", "priority": 0},
  {"ssid": "cluster2", "password": "isms12345@", "priority": 1},
  {"ssid": "cluster3", "password": "isms12345@", "priority": 2},
  {"ssid": "cluster4", "password": "isms12345@", "priority": 3},
  {"ssid": "cluster5", "password": "isms12345@", "priority": 4},
  {"ssid": "cluster6", "password": "isms12345@", "priority": 5}
]
EOF
    chown "$APP_USER:$APP_USER" "$DATA_DIR/wifi_configs.json"
fi
log_info "Configuration files ready"

# Install and start systemd service
log_info "[11/11] Installing systemd service..."
cp "$SCRIPT_DIR/is20s.service" /etc/systemd/system/
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"
sleep 5

# Verify
if systemctl is-active --quiet "$SERVICE_NAME"; then
    # Health check
    HEALTH=$(curl -s http://localhost:8080/health 2>/dev/null || echo '{"ok":false}')

    log_info ""
    log_info "=========================================="
    log_info "  Installation Complete!"
    log_info "=========================================="
    log_info "Service: $(systemctl is-active $SERVICE_NAME)"
    log_info "Health:  $HEALTH"
    log_info "Web UI:  http://$(hostname -I | awk '{print $1}'):8080"
    log_info ""
    log_info "Commands:"
    log_info "  sudo systemctl status is20s"
    log_info "  sudo systemctl restart is20s"
    log_info "  sudo journalctl -u is20s -f"
    log_info "=========================================="
else
    log_error "Service failed to start!"
    log_error "Check logs: journalctl -u is20s -n 50"
    exit 1
fi
