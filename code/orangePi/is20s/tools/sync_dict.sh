#!/bin/bash
# akbの辞書を tam/to に同期し、ローカルバックアップを作成
# Usage: ./sync_dict.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKUP_DIR="$SCRIPT_DIR/../dict_backups"
TIMESTAMP=$(date +%Y%m%d%H%M%S)
BACKUP_FILE="$BACKUP_DIR/domain_services_$TIMESTAMP.json"

AKB="192.168.3.250"
TAM="192.168.125.248"
TO="192.168.126.248"
PASS="mijeos12345@"

echo "=== 辞書同期 ==="
echo ""

# 1. akbから辞書取得・ローカルバックアップ
echo "1. akbから辞書取得..."
SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@$AKB \
    "cat /var/lib/is20s/domain_services.json" > "$BACKUP_FILE"

PATTERN_COUNT=$(python3 -c "import json; print(len(json.load(open('$BACKUP_FILE')).get('services', {})))")
echo "   パターン数: $PATTERN_COUNT"
echo "   バックアップ: $BACKUP_FILE"

# 2. tam に同期
echo ""
echo "2. tam ($TAM) に同期..."
SSHPASS="$PASS" sshpass -e scp -o StrictHostKeyChecking=no "$BACKUP_FILE" mijeosadmin@$TAM:/tmp/domain_services.json
SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@$TAM \
    "echo '$PASS' | sudo -S cp /tmp/domain_services.json /var/lib/is20s/domain_services.json 2>/dev/null && echo '$PASS' | sudo -S systemctl restart is20s 2>/dev/null"
echo "   ✓ tam 完了"

# 3. to に同期
echo ""
echo "3. to ($TO) に同期..."
SSHPASS="$PASS" sshpass -e scp -o StrictHostKeyChecking=no "$BACKUP_FILE" mijeosadmin@$TO:/tmp/domain_services.json
SSHPASS="$PASS" sshpass -e ssh -o StrictHostKeyChecking=no mijeosadmin@$TO \
    "echo '$PASS' | sudo -S cp /tmp/domain_services.json /var/lib/is20s/domain_services.json 2>/dev/null && echo '$PASS' | sudo -S systemctl restart is20s 2>/dev/null"
echo "   ✓ to 完了"

# 4. 確認
echo ""
echo "4. 確認..."
sleep 3
AKB_COUNT=$(curl -s "http://$AKB:8080/api/domain-services" | python3 -c "import json,sys; print(len(json.load(sys.stdin).get('services', {})))")
TAM_COUNT=$(curl -s "http://$TAM:8080/api/domain-services" | python3 -c "import json,sys; print(len(json.load(sys.stdin).get('services', {})))")
TO_COUNT=$(curl -s "http://$TO:8080/api/domain-services" | python3 -c "import json,sys; print(len(json.load(sys.stdin).get('services', {})))")

echo "   akb: $AKB_COUNT patterns"
echo "   tam: $TAM_COUNT patterns"
echo "   to:  $TO_COUNT patterns"

echo ""
echo "=== 同期完了 ==="
echo ""
echo "※ 必ず git add & commit & push してください:"
echo "   cd $(dirname $BACKUP_DIR)"
echo "   git add dict_backups/"
echo "   git commit -m 'dict: update domain_services ($PATTERN_COUNT patterns)'"
echo "   git push"
