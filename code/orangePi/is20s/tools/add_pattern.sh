#!/bin/bash
# パターンを追加（akbのみ。同期は別途sync_dict.shで行う）
# Usage: ./add_pattern.sh <pattern> <service> <category>
# Example: ./add_pattern.sh "kurogames" "Kuro Games" "Game"

set -e

if [ $# -lt 3 ]; then
    echo "Usage: $0 <pattern> <service> <category>"
    echo "Example: $0 kurogames 'Kuro Games' Game"
    exit 1
fi

PATTERN="$1"
SERVICE="$2"
CATEGORY="$3"

AKB="192.168.3.250"

echo "Adding: $PATTERN -> $SERVICE ($CATEGORY)"

RESULT=$(curl -s -X POST "http://$AKB:8080/api/domain-services" \
    -H "Content-Type: application/json" \
    -d "{\"pattern\":\"$PATTERN\",\"service\":\"$SERVICE\",\"category\":\"$CATEGORY\"}")

OK=$(echo "$RESULT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('ok', False))")

if [ "$OK" = "True" ]; then
    echo "✓ 追加成功"
else
    echo "✗ 追加失敗: $RESULT"
    exit 1
fi

echo ""
echo "※ 追加完了後、sync_dict.sh で3デバイスに同期してください"
