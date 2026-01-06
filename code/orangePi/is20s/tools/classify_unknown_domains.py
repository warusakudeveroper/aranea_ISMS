#!/usr/bin/env python3
"""
不明ドメイン自動分類スクリプト
300件の不明ドメインをカテゴリ・サービス名に分類
"""
import json
import sys
from pathlib import Path
from typing import Dict, Tuple

# カテゴリ分類ルール（優先順位順）
CLASSIFICATION_RULES = [
    # Gaming
    (["yuanshen", "genshin", "mihoyo", "hoyoverse"], "Gaming", "Genshin Impact (miHoYo)"),
    (["starrails", "honkai"], "Gaming", "Honkai: Star Rail (miHoYo)"),
    (["zenlesszonezero", "zenless"], "Gaming", "Zenless Zone Zero (miHoYo)"),
    (["kurogame", "mingchao", "wuthering"], "Gaming", "Wuthering Waves (Kuro Games)"),
    (["yostar"], "Gaming", "Yostar Games"),
    (["game", "gaming"], "Gaming", "Gaming"),

    # AI/ML
    (["gliacloud"], "AI", "GliaCloud (AI Video)"),
    (["openai", "anthropic"], "AI", "AI Service"),

    # Monitoring & Observability
    (["nr-data.net", "newrelic"], "Monitoring", "New Relic"),
    (["sentry.io"], "Monitoring", "Sentry"),
    (["honeycomb.io"], "Monitoring", "Honeycomb"),

    # Tracking & Analytics
    (["keepa.com"], "Tracker", "Keepa (Price Tracker)"),
    (["app-measurement.com"], "Analytics", "Google Analytics"),
    (["analytics", "measurement", "metrics"], "Analytics", "Analytics"),
    (["pixel", "tracking", "tracker"], "Tracker", "Tracker"),

    # Booking & Travel
    (["booking.com", "beds24.com"], "Booking", "Booking Service"),

    # IoT
    (["daikinsmartdb.jp", "daikin"], "IoT", "Daikin Smart"),
    (["iot", "smart"], "IoT", "IoT"),

    # Ad Network
    (["adnexus", "doubleclick", "ad-stir", "ad.as", "amanad", "socdm", "adtdp"], "Ad", "Ad Network"),
    (["youboranqs01.com"], "Ad", "Ad Network"),
    (["creativecdn.com"], "Ad", "Creative CDN"),
    (["uncn.jp"], "Ad", "Ad Network (Japan)"),
    (["a-dsp.com"], "Ad", "DSP"),
    (["nielsen.com"], "Ad", "Nielsen"),
    (["onelink.me"], "Ad", "AppsFlyer OneLink"),

    # AWS Services
    (["diagnostic.networking.aws.dev"], "Network", "AWS Network Diagnostics"),
    (["awswaf.com"], "Security", "AWS WAF"),
    (["cloudfront"], "CDN", "AWS CloudFront"),
    (["amazonaws.com"], "Cloud", "AWS"),

    # CDN
    (["cdn", "cloudflare", "akamai", "fastly"], "CDN", "CDN"),
    (["byteglb.com"], "CDN", "ByteDance Global CDN"),
    (["kunlunhuf.com", "lahuashanbx.com"], "CDN", "China CDN"),
    (["bdydns.com", "bdstatic.com"], "CDN", "Baidu CDN"),

    # Security
    (["mcafee.com"], "Security", "McAfee"),
    (["hcaptcha.com"], "Security", "hCaptcha"),
    (["waf"], "Security", "WAF"),

    # Payment
    (["stripe.com"], "Payment", "Stripe"),
    (["americanexpress.com"], "Payment", "American Express"),
    (["paypal"], "Payment", "PayPal"),

    # Microsoft
    (["msidentity.com", "msa.msidentity"], "Microsoft", "Microsoft Identity"),
    (["sharepoint.com"], "Microsoft", "SharePoint"),
    (["1drv.com", "onedrive"], "Microsoft", "OneDrive"),
    (["microsoft"], "Microsoft", "Microsoft"),

    # Social Media
    (["t.co"], "Social", "Twitter (X)"),
    (["ameblo.jp"], "Blog", "Ameba Blog"),
    (["twitter", "facebook", "instagram"], "Social", "Social Media"),

    # Development Tools
    (["vscode-cdn.net"], "Development", "VS Code"),
    (["obsidian.md"], "Development", "Obsidian"),
    (["github"], "Development", "GitHub"),
    (["growthbook.io"], "Development", "GrowthBook"),

    # Xiaomi / MIUI
    (["miui.com", "xiaomi"], "Xiaomi", "Xiaomi/MIUI"),
    (["n.shifen.com"], "CDN", "Baidu CDN (Shifen)"),

    # News & Media
    (["newspicks.com"], "News", "NewsPicks"),
    (["hjholdings.tv"], "Media", "HJ Holdings"),

    # Hosting
    (["vultrusercontent.com"], "Hosting", "Vultr"),
    (["lolipop.jp"], "Hosting", "Lolipop"),

    # Network Infrastructure
    (["ipv4only.arpa"], "Network", "IPv4 Test"),

    # E-commerce
    (["amazon", "rakuten", "yahoo"], "Shopping", "E-commerce"),

    # Cloud Functions
    (["cloudfunctions.net"], "Cloud", "Google Cloud Functions"),
    (["force.com"], "Cloud", "Salesforce"),

    # Messaging
    (["im-apps.net"], "Messaging", "Messaging Service"),

    # Telecom
    (["ntt", "softbank", "docomo"], "Telecom", "Telecom"),
]

def classify_domain(domain: str) -> Tuple[str, str]:
    """
    ドメインをカテゴリとサービス名に分類

    Returns:
        (category, service) のタプル
    """
    domain_lower = domain.lower()

    for keywords, category, service in CLASSIFICATION_RULES:
        if any(kw in domain_lower for kw in keywords):
            return (category, service)

    # 分類不可
    return ("Unknown", "Unknown")

def main():
    input_file = Path(__file__).parent / "unknown_domains_20260106.json"
    output_file = Path(__file__).parent / "classified_domains_20260106.json"

    if not input_file.exists():
        print(f"Error: {input_file} not found", file=sys.stderr)
        sys.exit(1)

    # 不明ドメインリスト読み込み
    with open(input_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # 分類実行
    classified = []
    category_stats = {}

    for item in data['domains']:
        domain = item['domain']
        count = item['count']
        category, service = classify_domain(domain)

        classified.append({
            "domain": domain,
            "count": count,
            "category": category,
            "service": service,
            "first_seen": item["first_seen"],
            "last_seen": item["last_seen"],
            "sample_ips": item["sample_ips"],
            "rooms": item["rooms"]
        })

        # 統計
        if category not in category_stats:
            category_stats[category] = {"count": 0, "total_hits": 0}
        category_stats[category]["count"] += 1
        category_stats[category]["total_hits"] += count

    # カテゴリ別にソート
    classified.sort(key=lambda x: (x["category"], -x["count"]))

    # 結果保存
    result = {
        "total_domains": len(classified),
        "total_hits": data["total_hits"],
        "category_stats": category_stats,
        "domains": classified
    }

    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    # 統計表示
    print(f"✓ 分類完了: {len(classified)}ドメイン")
    print(f"\n【カテゴリ別統計】")
    for cat, stats in sorted(category_stats.items(), key=lambda x: -x[1]["total_hits"]):
        print(f"  {cat}: {stats['count']}ドメイン ({stats['total_hits']:,}ヒット)")

    print(f"\n保存先: {output_file}")

if __name__ == "__main__":
    main()
