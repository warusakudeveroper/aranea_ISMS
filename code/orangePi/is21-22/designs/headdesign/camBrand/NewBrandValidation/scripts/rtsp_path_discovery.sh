#!/bin/bash
#
# RTSPパス自動探索スクリプト
# 一般的なRTSPパスを試行し、動作するものを特定する
#
# Usage:
#   ./rtsp_path_discovery.sh --ip 192.168.1.100 --user admin --pass password
#

set -e

# デフォルト値
CAMERA_IP=""
RTSP_USER=""
RTSP_PASS=""
RTSP_PORT=554

# カラー出力
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ヘルプ
show_help() {
    cat << EOF
RTSPパス自動探索スクリプト

Usage: $0 [OPTIONS]

Required:
  --ip IP           カメラIPアドレス
  --user USERNAME   RTSPユーザー名
  --pass PASSWORD   RTSPパスワード

Optional:
  --port PORT       RTSPポート (default: 554)
  --help            このヘルプを表示

Example:
  $0 --ip 192.168.1.100 --user admin --pass test123
EOF
    exit 0
}

# 引数解析
while [[ $# -gt 0 ]]; do
    case $1 in
        --ip) CAMERA_IP="$2"; shift 2 ;;
        --user) RTSP_USER="$2"; shift 2 ;;
        --pass) RTSP_PASS="$2"; shift 2 ;;
        --port) RTSP_PORT="$2"; shift 2 ;;
        --help) show_help ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# 必須引数チェック
if [[ -z "$CAMERA_IP" || -z "$RTSP_USER" || -z "$RTSP_PASS" ]]; then
    echo "Error: --ip, --user, --pass are required"
    exit 1
fi

# パスワードのURLエンコード
ENCODED_PASS=$(echo -n "$RTSP_PASS" | python3 -c "import urllib.parse; import sys; print(urllib.parse.quote(sys.stdin.read(), safe=''))")

# 一般的なRTSPパス一覧
RTSP_PATHS=(
    # TP-link Tapo/VIGI
    "/stream1"
    "/stream2"

    # NVT/JOOAN/CAM720系
    "/live/ch00_0"
    "/live/ch00_1"
    "/live/ch01_0"
    "/live/ch01_1"

    # Hikvision
    "/Streaming/Channels/101"
    "/Streaming/Channels/102"
    "/Streaming/Channels/201"
    "/Streaming/Channels/202"
    "/h264/ch1/main/av_stream"
    "/h264/ch1/sub/av_stream"

    # Dahua
    "/cam/realmonitor?channel=1&subtype=0"
    "/cam/realmonitor?channel=1&subtype=1"

    # ONVIF標準
    "/onvif1"
    "/onvif2"
    "/MediaInput/h264"
    "/MediaInput/h264/stream_1"

    # 汎用
    "/live"
    "/live/main"
    "/live/sub"
    "/video1"
    "/video2"
    "/1"
    "/2"
    "/11"
    "/12"
    "/ch0_0.h264"
    "/ch0_1.h264"
    "/av0_0"
    "/av0_1"

    # Axis
    "/axis-media/media.amp"
    "/axis-media/media.amp?videocodec=h264"

    # Foscam
    "/videoMain"
    "/videoSub"

    # Reolink
    "/h264Preview_01_main"
    "/h264Preview_01_sub"

    # Amcrest
    "/cam/realmonitor?channel=1&subtype=0&unicast=true&proto=Onvif"
)

echo "========================================"
echo "RTSPパス自動探索"
echo "========================================"
echo "カメラIP: $CAMERA_IP"
echo "ポート: $RTSP_PORT"
echo "ユーザー: $RTSP_USER"
echo "========================================"
echo ""

found_paths=()

for path in "${RTSP_PATHS[@]}"; do
    url="rtsp://${RTSP_USER}:${ENCODED_PASS}@${CAMERA_IP}:${RTSP_PORT}${path}"

    # 進捗表示
    printf "Testing: %-50s ... " "$path"

    # ffprobeで接続テスト
    result=$(timeout 5 ffprobe -rtsp_transport tcp -v error \
        -show_entries stream=width,height,codec_name -of csv=p=0 \
        "$url" 2>/dev/null)

    if [[ -n "$result" ]]; then
        echo -e "${GREEN}SUCCESS${NC} ($result)"
        found_paths+=("$path|$result")
    else
        echo -e "${RED}FAILED${NC}"
    fi
done

echo ""
echo "========================================"
echo "検出されたパス"
echo "========================================"

if [[ ${#found_paths[@]} -eq 0 ]]; then
    echo -e "${YELLOW}動作するパスが見つかりませんでした${NC}"
    echo ""
    echo "確認事項:"
    echo "  1. ユーザー名の大文字小文字を確認（Admin vs admin）"
    echo "  2. パスワードが正しいか確認"
    echo "  3. カメラのRTSP機能が有効か確認"
    echo "  4. ファイアウォール設定を確認"
else
    # 解像度でソート（メイン/サブ判定）
    echo ""
    echo "| パス | 解像度 | コーデック | 推定 |"
    echo "|------|--------|-----------|------|"

    # 最大解像度を特定
    max_width=0
    for entry in "${found_paths[@]}"; do
        path=$(echo "$entry" | cut -d'|' -f1)
        info=$(echo "$entry" | cut -d'|' -f2)
        width=$(echo "$info" | cut -d',' -f1)

        if [[ $width -gt $max_width ]]; then
            max_width=$width
        fi
    done

    for entry in "${found_paths[@]}"; do
        path=$(echo "$entry" | cut -d'|' -f1)
        info=$(echo "$entry" | cut -d'|' -f2)
        width=$(echo "$info" | cut -d',' -f1)
        height=$(echo "$info" | cut -d',' -f2)
        codec=$(echo "$info" | cut -d',' -f3)

        if [[ $width -eq $max_width ]]; then
            stream_type="メイン"
        else
            stream_type="サブ"
        fi

        echo "| $path | ${width}x${height} | $codec | $stream_type |"
    done

    echo ""
    echo "========================================"
    echo "推奨設定"
    echo "========================================"

    # メインパスを特定
    for entry in "${found_paths[@]}"; do
        path=$(echo "$entry" | cut -d'|' -f1)
        width=$(echo "$entry" | cut -d'|' -f2 | cut -d',' -f1)

        if [[ $width -eq $max_width ]]; then
            echo "メインストリーム: $path"
            break
        fi
    done

    # サブパスを特定
    for entry in "${found_paths[@]}"; do
        path=$(echo "$entry" | cut -d'|' -f1)
        width=$(echo "$entry" | cut -d'|' -f2 | cut -d',' -f1)

        if [[ $width -ne $max_width ]]; then
            echo "サブストリーム: $path"
            break
        fi
    done
fi

echo ""
echo "========================================"
