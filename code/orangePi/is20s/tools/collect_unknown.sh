#!/bin/bash
# 3デバイスから不明ドメインを収集・マージ
# Usage: ./collect_unknown.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/../dict_backups"

AKB="192.168.3.250"
TAM="192.168.125.248"
TO="192.168.126.249"

echo "=== 不明ドメイン収集 ==="
echo ""

# 収集
echo "akb ($AKB)..."
curl -s "http://$AKB:8080/api/unknown-domains" > /tmp/unknown_akb.json
echo "tam ($TAM)..."
curl -s "http://$TAM:8080/api/unknown-domains" > /tmp/unknown_tam.json
echo "to ($TO)..."
curl -s "http://$TO:8080/api/unknown-domains" > /tmp/unknown_to.json

# マージ・表示
python3 << 'EOF'
import json
from collections import defaultdict

all_domains = defaultdict(lambda: {"count": 0, "sources": []})

for name, path in [("akb", "/tmp/unknown_akb.json"), ("tam", "/tmp/unknown_tam.json"), ("to", "/tmp/unknown_to.json")]:
    try:
        with open(path) as f:
            data = json.load(f)
            domains = data.get("domains", [])
            print(f"  {name}: {len(domains)} domains")
            for d in domains:
                dom = d.get("domain", "")
                if dom:
                    all_domains[dom]["count"] += d.get("count", 1)
                    all_domains[dom]["sources"].append(name)
    except Exception as e:
        print(f"  {name}: error - {e}")

sorted_domains = sorted(all_domains.items(), key=lambda x: -x[1]["count"])
print(f"\n=== 全{len(sorted_domains)}件 ===\n")
print(f"{'Count':>6} | {'Sources':<12} | Domain")
print("-" * 70)

for dom, info in sorted_domains:
    sources = ",".join(info["sources"])
    print(f"{info['count']:6d} | {sources:<12} | {dom}")
EOF

echo ""
echo "収集完了。上記ドメインを手動で分類してください。"
echo "分類後、add_pattern.sh で追加してください。"
