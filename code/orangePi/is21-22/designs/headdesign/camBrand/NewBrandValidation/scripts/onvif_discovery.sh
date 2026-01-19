#!/bin/bash
#
# ONVIFポート・サービス探索スクリプト
# 複数ポートでONVIFエンドポイントを試行し、動作するものを特定する
#
# Usage:
#   ./onvif_discovery.sh --ip 192.168.1.100
#

set -e

# デフォルト値
CAMERA_IP=""
USER=""
PASS=""

# カラー出力
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ヘルプ
show_help() {
    cat << EOF
ONVIFポート・サービス探索スクリプト

Usage: $0 [OPTIONS]

Required:
  --ip IP           カメラIPアドレス

Optional:
  --user USERNAME   ONVIF認証ユーザー名（認証が必要な場合）
  --pass PASSWORD   ONVIF認証パスワード（認証が必要な場合）
  --help            このヘルプを表示

Example:
  $0 --ip 192.168.1.100
  $0 --ip 192.168.1.100 --user admin --pass test123
EOF
    exit 0
}

# 引数解析
while [[ $# -gt 0 ]]; do
    case $1 in
        --ip) CAMERA_IP="$2"; shift 2 ;;
        --user) USER="$2"; shift 2 ;;
        --pass) PASS="$2"; shift 2 ;;
        --help) show_help ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# 必須引数チェック
if [[ -z "$CAMERA_IP" ]]; then
    echo "Error: --ip is required"
    exit 1
fi

# ONVIFポート一覧
ONVIF_PORTS=(
    80
    2020
    8080
    8899
    5000
    8000
    8081
    10080
)

# ONVIFエンドポイントパス一覧
ONVIF_PATHS=(
    "/onvif/device_service"
    "/onvif/services"
    "/onvif-http/snapshot"
)

# SOAPリクエスト（認証なし）
SOAP_GET_DEVICE_INFO='<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>'

SOAP_GET_CAPABILITIES='<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetCapabilities xmlns="http://www.onvif.org/ver10/device/wsdl"><Category>All</Category></GetCapabilities></s:Body></s:Envelope>'

echo "========================================"
echo "ONVIF探索"
echo "========================================"
echo "カメラIP: $CAMERA_IP"
echo "========================================"
echo ""

# ポートスキャン
echo "--- ポートスキャン ---"
open_ports=()
for port in "${ONVIF_PORTS[@]}"; do
    if nc -zv -w 2 "$CAMERA_IP" "$port" 2>/dev/null; then
        echo -e "Port $port: ${GREEN}OPEN${NC}"
        open_ports+=($port)
    else
        echo -e "Port $port: ${RED}CLOSED${NC}"
    fi
done
echo ""

if [[ ${#open_ports[@]} -eq 0 ]]; then
    echo -e "${YELLOW}開いているONVIFポートが見つかりませんでした${NC}"
    exit 1
fi

# ONVIFエンドポイント探索
echo "--- ONVIFエンドポイント探索 ---"
found_endpoints=()

for port in "${open_ports[@]}"; do
    for path in "${ONVIF_PATHS[@]}"; do
        url="http://${CAMERA_IP}:${port}${path}"
        printf "Testing: %-60s ... " "$url"

        response=$(curl -s --max-time 5 -X POST \
            -H "Content-Type: application/soap+xml" \
            -d "$SOAP_GET_DEVICE_INFO" \
            "$url" 2>/dev/null || true)

        if echo "$response" | grep -qiE "Manufacturer|tds:Manufacturer|GetDeviceInformationResponse"; then
            echo -e "${GREEN}SUCCESS${NC}"
            found_endpoints+=("$url")

            # 詳細情報抽出
            manufacturer=$(echo "$response" | grep -oP '(?<=Manufacturer>)[^<]+' | head -1 || echo "N/A")
            model=$(echo "$response" | grep -oP '(?<=Model>)[^<]+' | head -1 || echo "N/A")
            firmware=$(echo "$response" | grep -oP '(?<=FirmwareVersion>)[^<]+' | head -1 || echo "N/A")

            echo "  Manufacturer: $manufacturer"
            echo "  Model: $model"
            echo "  Firmware: $firmware"
        elif echo "$response" | grep -qiE "Fault|Unauthorized|ActionNotSupported"; then
            echo -e "${YELLOW}AUTH REQUIRED${NC}"
            found_endpoints+=("$url|AUTH_REQUIRED")
        else
            echo -e "${RED}FAILED${NC}"
        fi
    done
done
echo ""

# 結果サマリー
echo "========================================"
echo "検出結果サマリー"
echo "========================================"

if [[ ${#found_endpoints[@]} -eq 0 ]]; then
    echo -e "${YELLOW}動作するONVIFエンドポイントが見つかりませんでした${NC}"
    echo ""
    echo "確認事項:"
    echo "  1. カメラのONVIF機能が有効か確認"
    echo "  2. ファームウェアでONVIFポートを確認"
    echo "  3. 認証が必要な場合は --user --pass を指定"
else
    echo ""
    echo "| ポート | パス | 状態 |"
    echo "|--------|------|------|"

    for entry in "${found_endpoints[@]}"; do
        if echo "$entry" | grep -q "AUTH_REQUIRED"; then
            url=$(echo "$entry" | cut -d'|' -f1)
            port=$(echo "$url" | grep -oP ':\d+' | tr -d ':')
            path=$(echo "$url" | grep -oP '/onvif.*')
            echo "| $port | $path | 要認証 |"
        else
            port=$(echo "$entry" | grep -oP ':\d+' | tr -d ':')
            path=$(echo "$entry" | grep -oP '/onvif.*')
            echo "| $port | $path | 動作 |"
        fi
    done

    echo ""
    echo "========================================"
    echo "推奨設定"
    echo "========================================"

    # 最初に成功したエンドポイント
    for entry in "${found_endpoints[@]}"; do
        if ! echo "$entry" | grep -q "AUTH_REQUIRED"; then
            port=$(echo "$entry" | grep -oP ':\d+' | tr -d ':')
            echo "ONVIFポート: $port"
            echo "ONVIFエンドポイント: $entry"
            break
        fi
    done
fi

echo ""
echo "========================================"
