#!/bin/bash
# ローカル辞書を3デバイス(akb/tam/to)にデプロイ
# ※ DOMAIN_REGISTRATION_GUIDE.md に準拠
#
# Usage: ./sync_dict.sh [options]
#   -n, --dry-run    実際にデプロイせず確認のみ
#   -v, --verify     デプロイ後のMD5検証を詳細表示
#   -h, --help       ヘルプ表示

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ローカル辞書ファイル（ソースオブトゥルース）
LOCAL_DICT="$SCRIPT_DIR/../data/default_domain_services.json"

# デプロイ先
REMOTE_PATH="/opt/is20s/data/default_domain_services.json"
RUNTIME_PATH="/var/lib/is20s/domain_services.json"

# デバイス情報
AKB="192.168.3.250"
TAM="192.168.125.248"
TO="192.168.126.248"
PASS="mijeos12345@"

# オプション解析
DRY_RUN=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--dry-run)
            DRY_RUN=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [options]"
            echo "  -n, --dry-run    実際にデプロイせず確認のみ"
            echo "  -v, --verbose    デプロイ後のMD5検証を詳細表示"
            echo "  -h, --help       ヘルプ表示"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== 辞書デプロイ ==="
echo ""

# 1. ローカル辞書確認
if [ ! -f "$LOCAL_DICT" ]; then
    echo "Error: ローカル辞書が見つかりません: $LOCAL_DICT"
    exit 1
fi

LOCAL_MD5=$(md5 -q "$LOCAL_DICT" 2>/dev/null || md5sum "$LOCAL_DICT" | cut -d' ' -f1)
PATTERN_COUNT=$(python3 -c "import json; print(len(json.load(open('$LOCAL_DICT')).get('services', {})))")

echo "1. ローカル辞書確認"
echo "   ファイル: $LOCAL_DICT"
echo "   パターン数: $PATTERN_COUNT"
echo "   MD5: $LOCAL_MD5"

if [ "$DRY_RUN" = true ]; then
    echo ""
    echo "[DRY-RUN] 以下のデバイスにデプロイされます:"
    echo "   - akb ($AKB)"
    echo "   - tam ($TAM)"
    echo "   - to ($TO)"
    echo ""
    echo "[DRY-RUN] 実際のデプロイは行いません"
    exit 0
fi

# デプロイ関数
deploy_to_device() {
    local NAME=$1
    local IP=$2
    local TIMEOUT=$3

    echo ""
    echo "2. $NAME ($IP) にデプロイ..."

    # SCP でデフォルト辞書をデプロイ
    SSHPASS="$PASS" sshpass -e scp -o StrictHostKeyChecking=no -o ConnectTimeout=$TIMEOUT \
        "$LOCAL_DICT" mijeosadmin@$IP:/tmp/default_domain_services.json

    # ランタイム辞書を削除してデフォルト辞書を配置、サービス再起動
    SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=$TIMEOUT mijeosadmin@$IP \
        "echo '$PASS' | sudo -S rm -f '$RUNTIME_PATH' 2>/dev/null; \
         echo '$PASS' | sudo -S cp /tmp/default_domain_services.json '$REMOTE_PATH' 2>/dev/null; \
         echo '$PASS' | sudo -S systemctl restart is20s 2>/dev/null"

    echo "   ✓ $NAME 完了"
}

# 2. akb にデプロイ
deploy_to_device "akb" "$AKB" 15

# 3. tam にデプロイ
deploy_to_device "tam" "$TAM" 15

# 4. to にデプロイ
deploy_to_device "to" "$TO" 20

# 5. 検証
echo ""
echo "5. 検証（3秒待機）..."
sleep 3

verify_device() {
    local NAME=$1
    local IP=$2
    local TIMEOUT=$3

    # リモートのデフォルト辞書のMD5を取得
    REMOTE_MD5=$(SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no -o ConnectTimeout=$TIMEOUT mijeosadmin@$IP \
        "md5sum '$REMOTE_PATH' 2>/dev/null | cut -d' ' -f1" 2>/dev/null || echo "ERROR")

    if [ "$REMOTE_MD5" = "$LOCAL_MD5" ]; then
        echo "   ✓ $NAME: OK (MD5一致)"
    else
        echo "   ✗ $NAME: NG (MD5不一致: $REMOTE_MD5)"
        return 1
    fi

    if [ "$VERBOSE" = true ]; then
        # API経由でパターン数も確認
        API_COUNT=$(curl -s --connect-timeout 5 "http://$IP:8080/api/domain-services" 2>/dev/null | \
            python3 -c "import json,sys; print(len(json.load(sys.stdin).get('services', {})))" 2>/dev/null || echo "ERROR")
        echo "     API確認: $API_COUNT patterns"
    fi
}

echo ""
echo "   ローカル MD5: $LOCAL_MD5"

VERIFY_OK=true
verify_device "akb" "$AKB" 15 || VERIFY_OK=false
verify_device "tam" "$TAM" 15 || VERIFY_OK=false
verify_device "to" "$TO" 20 || VERIFY_OK=false

echo ""
if [ "$VERIFY_OK" = true ]; then
    echo "=== デプロイ完了 (全デバイスMD5一致) ==="
else
    echo "=== デプロイ完了 (※一部検証失敗) ==="
    echo ""
    echo "失敗したデバイスは手動確認してください"
fi

echo ""
echo "※ 辞書を変更した場合は git commit & push してください:"
echo "   git add data/default_domain_services.json"
echo "   git commit -m 'dict: update domain_services ($PATTERN_COUNT patterns)'"
echo "   git push"
