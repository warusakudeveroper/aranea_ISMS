#!/bin/bash
#
# カメラストレステストスクリプト
# 新規カメラブランド検証用
#
# Usage:
#   ./camera_stress_test.sh --ip 192.168.1.100 --user admin --pass password \
#     --main-path /stream1 --sub-path /stream2 --onvif-port 2020
#

set -e

# デフォルト値
CAMERA_IP=""
RTSP_USER=""
RTSP_PASS=""
MAIN_PATH="/stream1"
SUB_PATH="/stream2"
RTSP_PORT=554
ONVIF_PORT=2020
OUTPUT_DIR="/tmp/camera_stress_test_$(date +%Y%m%d_%H%M%S)"

# カラー出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# ヘルプ
show_help() {
    cat << EOF
カメラストレステストスクリプト

Usage: $0 [OPTIONS]

Required:
  --ip IP           カメラIPアドレス
  --user USERNAME   RTSPユーザー名
  --pass PASSWORD   RTSPパスワード

Optional:
  --main-path PATH  メインストリームパス (default: /stream1)
  --sub-path PATH   サブストリームパス (default: /stream2)
  --rtsp-port PORT  RTSPポート (default: 554)
  --onvif-port PORT ONVIFポート (default: 2020)
  --output-dir DIR  出力ディレクトリ (default: /tmp/camera_stress_test_YYYYMMDD_HHMMSS)
  --help            このヘルプを表示

Example:
  $0 --ip 192.168.1.100 --user admin --pass test123 \\
     --main-path /live/ch00_0 --sub-path /live/ch00_1 --onvif-port 8899
EOF
    exit 0
}

# 引数解析
while [[ $# -gt 0 ]]; do
    case $1 in
        --ip) CAMERA_IP="$2"; shift 2 ;;
        --user) RTSP_USER="$2"; shift 2 ;;
        --pass) RTSP_PASS="$2"; shift 2 ;;
        --main-path) MAIN_PATH="$2"; shift 2 ;;
        --sub-path) SUB_PATH="$2"; shift 2 ;;
        --rtsp-port) RTSP_PORT="$2"; shift 2 ;;
        --onvif-port) ONVIF_PORT="$2"; shift 2 ;;
        --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --help) show_help ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# 必須引数チェック
if [[ -z "$CAMERA_IP" || -z "$RTSP_USER" || -z "$RTSP_PASS" ]]; then
    echo "Error: --ip, --user, --pass are required"
    exit 1
fi

# URL構築（パスワードのURLエンコード）
ENCODED_PASS=$(echo -n "$RTSP_PASS" | python3 -c "import urllib.parse; import sys; print(urllib.parse.quote(sys.stdin.read(), safe=''))")
MAIN_URL="rtsp://${RTSP_USER}:${ENCODED_PASS}@${CAMERA_IP}:${RTSP_PORT}${MAIN_PATH}"
SUB_URL="rtsp://${RTSP_USER}:${ENCODED_PASS}@${CAMERA_IP}:${RTSP_PORT}${SUB_PATH}"
ONVIF_URL="http://${CAMERA_IP}:${ONVIF_PORT}/onvif/device_service"

# 出力ディレクトリ作成
mkdir -p "$OUTPUT_DIR"
REPORT_FILE="$OUTPUT_DIR/stress_test_report.txt"

# ログ関数
log() {
    local msg="$1"
    echo -e "$msg" | tee -a "$REPORT_FILE"
}

log_success() {
    log "${GREEN}✅ $1${NC}"
}

log_fail() {
    log "${RED}❌ $1${NC}"
}

log_warn() {
    log "${YELLOW}⚠️ $1${NC}"
}

# テスト関数: RTSP単発接続
test_single_rtsp() {
    local url="$1"
    local timeout_sec="${2:-5}"

    timeout $timeout_sec ffprobe -rtsp_transport tcp -v error \
        -show_entries stream=width,height -of csv=p=0 "$url" 2>/dev/null
}

# ========================================
# メイン処理開始
# ========================================

log "========================================"
log "カメラストレステスト"
log "========================================"
log "実行日時: $(date)"
log "カメラIP: $CAMERA_IP"
log "RTSPユーザー: $RTSP_USER"
log "メインパス: $MAIN_PATH"
log "サブパス: $SUB_PATH"
log "ONVIFポート: $ONVIF_PORT"
log "出力ディレクトリ: $OUTPUT_DIR"
log ""

# ========================================
# Test 1: 基本接続テスト
# ========================================
log "========================================"
log "Test 1: 基本接続テスト"
log "========================================"

log "--- メインストリーム接続テスト ---"
result=$(test_single_rtsp "$MAIN_URL" 10)
if [[ -n "$result" ]]; then
    log_success "メインストリーム: SUCCESS ($result)"
    MAIN_RESOLUTION="$result"
else
    log_fail "メインストリーム: FAILED"
    MAIN_RESOLUTION="N/A"
fi

log "--- サブストリーム接続テスト ---"
result=$(test_single_rtsp "$SUB_URL" 10)
if [[ -n "$result" ]]; then
    log_success "サブストリーム: SUCCESS ($result)"
    SUB_RESOLUTION="$result"
else
    log_fail "サブストリーム: FAILED"
    SUB_RESOLUTION="N/A"
fi
log ""

# ========================================
# Test 2: 再接続テスト
# ========================================
log "========================================"
log "Test 2: 再接続テスト"
log "========================================"

# 急速再接続（インターバルなし）
log "--- 急速再接続（インターバルなし）20回 ---"
rapid_success=0
for i in $(seq 1 20); do
    if test_single_rtsp "$MAIN_URL" 3 | grep -qE "^[0-9]+"; then
        rapid_success=$((rapid_success + 1))
    fi
done
rapid_rate=$((rapid_success * 100 / 20))
log "結果: $rapid_success/20 (${rapid_rate}%)"
if [[ $rapid_rate -ge 80 ]]; then
    log_success "急速再接続: 良好"
else
    log_warn "急速再接続: 制限あり（インターバル推奨）"
fi
log ""

sleep 2

# 0.5秒インターバル
log "--- 0.5秒インターバル 10回 ---"
int05_success=0
for i in $(seq 1 10); do
    if test_single_rtsp "$MAIN_URL" 3 | grep -qE "^[0-9]+"; then
        int05_success=$((int05_success + 1))
    fi
    sleep 0.5
done
log "結果: $int05_success/10"
log ""

sleep 2

# 1秒インターバル
log "--- 1秒インターバル 10回 ---"
int1_success=0
for i in $(seq 1 10); do
    if test_single_rtsp "$MAIN_URL" 3 | grep -qE "^[0-9]+"; then
        int1_success=$((int1_success + 1))
    fi
    sleep 1
done
log "結果: $int1_success/10"
log ""

sleep 2

# 2秒インターバル
log "--- 2秒インターバル 10回 ---"
int2_success=0
for i in $(seq 1 10); do
    if test_single_rtsp "$MAIN_URL" 3 | grep -qE "^[0-9]+"; then
        int2_success=$((int2_success + 1))
    fi
    sleep 2
done
log "結果: $int2_success/10"
log ""

# ========================================
# Test 3: 並列接続テスト
# ========================================
log "========================================"
log "Test 3: 並列接続テスト"
log "========================================"

test_concurrent() {
    local count=$1
    local success=0
    local total=$((count * 3))  # 3回試行

    for run in 1 2 3; do
        local run_success=0
        for i in $(seq 1 $count); do
            (test_single_rtsp "$MAIN_URL" 5 | grep -qE "^[0-9]+" && exit 0 || exit 1) &
        done

        # 全プロセスの終了を待機して結果をカウント
        for job in $(jobs -p); do
            wait $job && run_success=$((run_success + 1)) || true
        done
        success=$((success + run_success))
        sleep 2
    done

    echo "$success/$total"
}

log "--- 2並列接続テスト ---"
concurrent2_result=$(test_concurrent 2)
log "結果: $concurrent2_result"
if echo "$concurrent2_result" | grep -q "6/6"; then
    log_success "2並列: 安定"
else
    log_warn "2並列: 不安定"
fi
log ""

sleep 2

log "--- 3並列接続テスト ---"
concurrent3_result=$(test_concurrent 3)
log "結果: $concurrent3_result"
if echo "$concurrent3_result" | grep -q "9/9"; then
    log_success "3並列: 安定"
else
    log_warn "3並列: 不安定"
fi
log ""

sleep 2

log "--- 5並列接続テスト ---"
concurrent5_result=$(test_concurrent 5)
log "結果: $concurrent5_result"
log ""

# ========================================
# Test 4: Main + Sub 同時接続テスト
# ========================================
log "========================================"
log "Test 4: Main + Sub 同時接続テスト"
log "========================================"

main_sub_success=0
for run in 1 2 3; do
    log "Run $run:"

    main_result=$(test_single_rtsp "$MAIN_URL" 5) &
    main_pid=$!
    sub_result=$(test_single_rtsp "$SUB_URL" 5) &
    sub_pid=$!

    wait $main_pid
    main_code=$?
    wait $sub_pid
    sub_code=$?

    main_ok="FAIL"
    sub_ok="FAIL"
    [[ $main_code -eq 0 && -n "$main_result" ]] && main_ok="OK"
    [[ $sub_code -eq 0 && -n "$sub_result" ]] && sub_ok="OK"

    log "  Main: $main_ok, Sub: $sub_ok"

    [[ "$main_ok" == "OK" && "$sub_ok" == "OK" ]] && main_sub_success=$((main_sub_success + 1))
    sleep 2
done

log "結果: $main_sub_success/3"
if [[ $main_sub_success -eq 3 ]]; then
    log_success "Main+Sub同時: 安定"
else
    log_warn "Main+Sub同時: 不安定または不可"
fi
log ""

# ========================================
# Test 5: 長時間ストリームテスト
# ========================================
log "========================================"
log "Test 5: 長時間ストリームテスト (30秒)"
log "========================================"

start=$(date +%s)
timeout 35 ffmpeg -rtsp_transport tcp -i "$MAIN_URL" -t 30 -f null - 2>&1 | tail -3
result=$?
end=$(date +%s)
duration=$((end - start))

if [[ $result -eq 0 ]]; then
    log_success "30秒ストリーム: SUCCESS (${duration}秒)"
else
    log_fail "30秒ストリーム: FAILED (code=$result, ${duration}秒)"
fi
log ""

# ========================================
# Test 6: ONVIFストレステスト
# ========================================
log "========================================"
log "Test 6: ONVIFストレステスト"
log "========================================"

SOAP_REQ='<?xml version="1.0" encoding="UTF-8"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>'

log "--- ONVIF急速リクエスト 20回 ---"
onvif_success=0
for i in $(seq 1 20); do
    response=$(curl -s --max-time 3 -X POST \
        -H "Content-Type: application/soap+xml" \
        -d "$SOAP_REQ" "$ONVIF_URL" 2>/dev/null)
    if echo "$response" | grep -qiE "manufacturer|model"; then
        onvif_success=$((onvif_success + 1))
    fi
done
log "結果: $onvif_success/20"
if [[ $onvif_success -ge 18 ]]; then
    log_success "ONVIF: 安定"
else
    log_warn "ONVIF: 不安定または要認証"
fi
log ""

# ========================================
# Test 7: HTTP エンドポイントテスト
# ========================================
log "========================================"
log "Test 7: HTTPエンドポイントテスト"
log "========================================"

log "--- HTTP急速リクエスト 20回 ---"
http_success=0
for i in $(seq 1 20); do
    code=$(curl -s --max-time 2 -o /dev/null -w "%{http_code}" "http://${CAMERA_IP}/" 2>/dev/null)
    if echo "$code" | grep -qE "^(200|301|302|401)"; then
        http_success=$((http_success + 1))
    fi
done
log "結果: $http_success/20"
log ""

# ========================================
# 結果サマリー
# ========================================
log "========================================"
log "結果サマリー"
log "========================================"

log ""
log "【基本情報】"
log "  メイン解像度: $MAIN_RESOLUTION"
log "  サブ解像度: $SUB_RESOLUTION"
log ""
log "【再接続テスト】"
log "  急速(0秒): ${rapid_success}/20 (${rapid_rate}%)"
log "  0.5秒間隔: ${int05_success}/10"
log "  1秒間隔: ${int1_success}/10"
log "  2秒間隔: ${int2_success}/10"
log ""
log "【並列接続テスト】"
log "  2並列: $concurrent2_result"
log "  3並列: $concurrent3_result"
log "  5並列: $concurrent5_result"
log "  Main+Sub同時: ${main_sub_success}/3"
log ""
log "【推奨Access Absorber設定】"

# 設定値の自動判定
if [[ $rapid_rate -ge 80 ]]; then
    recommended_interval=0
elif [[ $int05_success -ge 8 ]]; then
    recommended_interval=500
elif [[ $int1_success -ge 8 ]]; then
    recommended_interval=1000
else
    recommended_interval=2000
fi

# 並列数の自動判定
if echo "$concurrent3_result" | grep -q "9/9"; then
    if echo "$concurrent5_result" | grep -qE "1[0-5]/15"; then
        recommended_concurrent=5
    else
        recommended_concurrent=3
    fi
elif echo "$concurrent2_result" | grep -q "6/6"; then
    recommended_concurrent=2
else
    recommended_concurrent=1
fi

# 排他ロックの判定
if [[ $recommended_concurrent -eq 1 ]]; then
    recommended_lock="TRUE"
else
    recommended_lock="FALSE"
fi

log "  max_concurrent_streams: $recommended_concurrent"
log "  min_reconnect_interval_ms: $recommended_interval"
log "  require_exclusive_lock: $recommended_lock"
log ""

# SQL出力
log "【DB投入SQL】"
log "INSERT INTO camera_access_limits"
log "(family, display_name, max_concurrent_streams, min_reconnect_interval_ms, require_exclusive_lock, notes)"
log "VALUES"
log "('NEW_FAMILY', 'Brand Name', $recommended_concurrent, $recommended_interval, $recommended_lock, 'Stress test $(date +%Y-%m-%d)');"
log ""

log "========================================"
log "テスト完了"
log "レポート: $REPORT_FILE"
log "========================================"
