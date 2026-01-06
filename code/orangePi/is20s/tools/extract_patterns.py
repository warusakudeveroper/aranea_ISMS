#!/usr/bin/env python3
"""
分類済みドメインから辞書パターン抽出
"""
import json
import sys
from pathlib import Path
from collections import defaultdict

def main():
    classified_file = Path(__file__).parent / "classified_domains_20260106.json"
    dict_file = Path(__file__).parent / "../data/default_domain_services.json"

    # 分類結果読み込み
    with open(classified_file, 'r', encoding='utf-8') as f:
        classified = json.load(f)

    # 既存辞書読み込み
    with open(dict_file, 'r', encoding='utf-8') as f:
        dictionary = json.load(f)

    existing_patterns = set(dictionary['services'].keys())

    # 新規パターン抽出（Unknownを除外）
    new_patterns = {}
    pattern_hits = defaultdict(int)

    for item in classified['domains']:
        domain = item['domain']
        category = item['category']
        service = item['service']
        count = item['count']

        # Unknownは除外
        if category == "Unknown":
            continue

        # 既に辞書にあるパターンは除外
        if domain in existing_patterns:
            continue

        # ドメインをパターンとして追加
        new_patterns[domain] = {
            "category": category,
            "service": service
        }
        pattern_hits[domain] = count

    # ヒット数でソート
    sorted_patterns = sorted(new_patterns.items(), key=lambda x: pattern_hits[x[0]], reverse=True)

    print(f"✓ 新規パターン抽出: {len(sorted_patterns)}件")
    print(f"  既存辞書: {len(existing_patterns)}パターン")
    print(f"  追加後: {len(existing_patterns) + len(sorted_patterns)}パターン")
    print(f"\n【新規パターン（上位20件）】")
    for i, (pattern, info) in enumerate(sorted_patterns[:20]):
        hits = pattern_hits[pattern]
        print(f"  {i+1:2d}. {pattern:40s} [{info['category']:10s}] {info['service']:30s} ({hits:,}ヒット)")

    # 辞書に追加
    for pattern, info in new_patterns.items():
        dictionary['services'][pattern] = info

    # 保存
    with open(dict_file, 'w', encoding='utf-8') as f:
        json.dump(dictionary, f, indent=2, ensure_ascii=False)

    print(f"\n✓ 辞書更新完了: {dict_file}")
    print(f"  合計: {len(dictionary['services'])}パターン (+{len(new_patterns)})")

if __name__ == "__main__":
    main()
