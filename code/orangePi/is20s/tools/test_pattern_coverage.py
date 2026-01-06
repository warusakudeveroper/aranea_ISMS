#!/usr/bin/env python3
"""
辞書パターンのカバレッジテスト（エミュレーションテスト）
追加した48パターンが実際の不明ドメイン300件をどれだけカバーできるかを検証
"""
import json
import sys
from pathlib import Path

def domain_matches_pattern(domain: str, pattern: str) -> bool:
    """
    ドメインがパターンにマッチするか判定
    domain_services.pyの実装と同じロジック
    """
    domain_lower = domain.lower()
    pattern_lower = pattern.lower()

    # 完全一致
    if domain_lower == pattern_lower:
        return True

    # サブドメインマッチ（例: api.example.com が example.com にマッチ）
    if domain_lower.endswith('.' + pattern_lower):
        return True

    # 部分一致（パターンがドメインに含まれる）
    if pattern_lower in domain_lower:
        return True

    return False

def main():
    # 新規追加パターン読み込み
    new_patterns_file = Path(__file__).parent / "new_patterns_20260106.json"
    with open(new_patterns_file, 'r') as f:
        new_patterns = json.load(f)

    # 不明ドメインリスト読み込み
    unknown_file = Path(__file__).parent / "unknown_domains_20260106.json"
    with open(unknown_file, 'r') as f:
        unknown_data = json.load(f)

    print(f"=== エミュレーションテスト ===\n")
    print(f"追加パターン数: {len(new_patterns)}")
    print(f"不明ドメイン数: {len(unknown_data['domains'])}")
    print(f"総ヒット数: {unknown_data['total_hits']:,}\n")

    # カバレッジテスト
    matched_domains = []
    matched_hits = 0
    unmatched_domains = []
    pattern_usage = {p['domain_pattern']: [] for p in new_patterns}

    for domain_entry in unknown_data['domains']:
        domain = domain_entry['domain']
        hits = domain_entry['count']
        matched = False

        for pattern_entry in new_patterns:
            pattern = pattern_entry['domain_pattern']

            if domain_matches_pattern(domain, pattern):
                matched = True
                matched_domains.append({
                    'domain': domain,
                    'hits': hits,
                    'pattern': pattern,
                    'category': pattern_entry['category'],
                    'service': pattern_entry['service']
                })
                matched_hits += hits
                pattern_usage[pattern].append(domain)
                break  # 最初にマッチしたパターンで終了

        if not matched:
            unmatched_domains.append({
                'domain': domain,
                'hits': hits
            })

    # 結果サマリー
    coverage_rate = (len(matched_domains) / len(unknown_data['domains'])) * 100
    hits_coverage_rate = (matched_hits / unknown_data['total_hits']) * 100

    print(f"【カバレッジ】")
    print(f"  マッチしたドメイン: {len(matched_domains)}/{len(unknown_data['domains'])} ({coverage_rate:.1f}%)")
    print(f"  マッチしたヒット数: {matched_hits:,}/{unknown_data['total_hits']:,} ({hits_coverage_rate:.1f}%)")
    print(f"  未マッチドメイン: {len(unmatched_domains)} ({100-coverage_rate:.1f}%)")

    # パターン使用統計
    print(f"\n【パターン使用状況】")
    used_patterns = {p: domains for p, domains in pattern_usage.items() if len(domains) > 0}
    unused_patterns = {p: domains for p, domains in pattern_usage.items() if len(domains) == 0}

    print(f"  使用されたパターン: {len(used_patterns)}/{len(new_patterns)}")
    print(f"  未使用パターン: {len(unused_patterns)}/{len(new_patterns)}")

    if unused_patterns:
        print(f"\n  ⚠️ 未使用パターン一覧:")
        for pattern in sorted(unused_patterns.keys()):
            pattern_info = next(p for p in new_patterns if p['domain_pattern'] == pattern)
            print(f"    - {pattern:40s} [{pattern_info['category']}] {pattern_info['service']}")

    # マッチ例表示（カテゴリ別）
    print(f"\n【マッチ例】（カテゴリ別）")
    from collections import defaultdict
    by_category = defaultdict(list)
    for m in matched_domains:
        by_category[m['category']].append(m)

    for cat in sorted(by_category.keys()):
        matches = by_category[cat]
        total_cat_hits = sum(m['hits'] for m in matches)
        print(f"\n  {cat} ({len(matches)}ドメイン / {total_cat_hits:,}ヒット):")
        for m in sorted(matches, key=lambda x: -x['hits'])[:5]:  # 上位5件
            print(f"    {m['domain']:50s} → {m['pattern']:30s} ({m['hits']:,}回)")

    # 未マッチ上位10件
    print(f"\n【未マッチドメイン】 上位10件（要手動分類）")
    for i, d in enumerate(sorted(unmatched_domains, key=lambda x: -x['hits'])[:10], 1):
        print(f"  {i:2d}. {d['domain']:60s} {d['hits']:6,}回")

    # 結果保存
    result = {
        "summary": {
            "total_patterns": len(new_patterns),
            "total_domains": len(unknown_data['domains']),
            "matched_domains": len(matched_domains),
            "unmatched_domains": len(unmatched_domains),
            "coverage_rate": coverage_rate,
            "matched_hits": matched_hits,
            "total_hits": unknown_data['total_hits'],
            "hits_coverage_rate": hits_coverage_rate
        },
        "matched": matched_domains,
        "unmatched": unmatched_domains,
        "pattern_usage": {p: len(domains) for p, domains in pattern_usage.items()}
    }

    output_file = Path(__file__).parent / "coverage_test_result_20260106.json"
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    print(f"\n保存先: {output_file}")

    # 判定
    if coverage_rate >= 80:
        print(f"\n✅ カバレッジ {coverage_rate:.1f}% - 良好")
    elif coverage_rate >= 60:
        print(f"\n⚠️  カバレッジ {coverage_rate:.1f}% - 改善推奨")
    else:
        print(f"\n❌ カバレッジ {coverage_rate:.1f}% - 要改善")

if __name__ == "__main__":
    main()
