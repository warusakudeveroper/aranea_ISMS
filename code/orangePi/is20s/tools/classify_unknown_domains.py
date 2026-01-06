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
    (["yuanshen", "genshin"], "Gaming", "Genshin Impact (miHoYo)"),
    (["starrails", "honkai"], "Gaming", "Honkai: Star Rail (miHoYo)"),
    (["zenlesszonezero", "zenless"], "Gaming", "Zenless Zone Zero (miHoYo)"),
    (["hoyolab.com", "mihoyo", "hoyoverse"], "Gaming", "miHoYo Community"),
    (["kurogame", "mingchao", "wuthering", "aki-game"], "Gaming", "Wuthering Waves (Kuro Games)"),
    (["yostar"], "Gaming", "Yostar Games"),
    (["wetest.net", "crashsight"], "Gaming", "Tencent Gaming Services"),
    (["ysdp.top"], "Gaming", "Gaming Service"),
    (["bluearchive.jp"], "Gaming", "Blue Archive"),
    (["launcher", "hggslb.com"], "Gaming", "Game Launcher"),
    (["game", "gaming"], "Gaming", "Gaming"),

    # AI/ML
    (["gliacloud"], "AI", "GliaCloud (AI Video)"),
    (["openai", "anthropic"], "AI", "AI Service"),

    # Monitoring & Observability
    (["nr-data.net", "newrelic"], "Monitoring", "New Relic"),
    (["sentry.io"], "Monitoring", "Sentry"),
    (["honeycomb.io"], "Monitoring", "Honeycomb"),
    (["datadog", "datadoghq"], "Monitoring", "Datadog"),
    (["usonar.jp", "sonar"], "Monitoring", "U-SONAR Analytics"),

    # Tracking & Analytics
    (["keepa.com"], "Tracker", "Keepa (Price Tracker)"),
    (["app-measurement.com"], "Analytics", "Google Analytics"),
    (["omtrdc.net", "omtr"], "Analytics", "Adobe Analytics"),
    (["imrworldwide.com", "comscore"], "Analytics", "Nielsen/comScore"),
    (["mieru-ca.com"], "Analytics", "Mieru-CA Analytics"),
    (["leanplum.com"], "Analytics", "Leanplum"),
    (["braze.com", "braze.eu"], "Analytics", "Braze"),
    (["appcenter.ms"], "Analytics", "App Center Analytics"),
    (["singular.net"], "Analytics", "Singular"),
    (["piyolog.com"], "Analytics", "Piyolog"),
    (["telemetry"], "Analytics", "Telemetry"),
    (["analytics", "measurement", "metrics"], "Analytics", "Analytics"),
    (["pixel", "tracking", "tracker"], "Tracker", "Tracker"),

    # Marketing Automation
    (["evergage.com"], "Marketing", "Evergage"),
    (["marketo.net"], "Marketing", "Marketo"),
    (["qualtrics.com"], "Marketing", "Qualtrics"),
    (["surveygizmo.eu"], "Marketing", "SurveyGizmo"),
    (["agkn.com"], "Marketing", "Neustar"),
    (["1rx.io"], "Marketing", "1rx"),

    # Booking & Travel
    (["booking.com", "beds24.com"], "Booking", "Booking Service"),

    # IoT
    (["daikinsmartdb.jp", "daikin"], "IoT", "Daikin Smart"),
    (["tplinkra.com", "tplink"], "IoT", "TP-Link Router"),
    (["iot", "smart"], "IoT", "IoT"),

    # Ad Network
    (["exp-tas.com", "exponential"], "Ad", "Exponential (Ad Network)"),
    (["youborafds", "youboranqs"], "Ad", "Youbora Ad Network"),
    (["adnexus", "doubleclick", "ad-stir", "ad.as", "amanad", "socdm", "adtdp"], "Ad", "Ad Network"),
    (["creativecdn.com"], "Ad", "Creative CDN"),
    (["uncn.jp"], "Ad", "Ad Network (Japan)"),
    (["a-dsp.com"], "Ad", "DSP"),
    (["nielsen.com"], "Ad", "Nielsen"),
    (["onelink.me"], "Ad", "AppsFlyer OneLink"),
    (["unrulymedia.com"], "Ad", "Unruly Media"),

    # AWS Services
    (["diagnostic.networking.aws.dev"], "Network", "AWS Network Diagnostics"),
    (["awswaf.com"], "Security", "AWS WAF"),
    (["cloudfront"], "CDN", "AWS CloudFront"),
    (["a2z.com"], "Cloud", "AWS (Amazon)"),
    (["amazonaws.com"], "Cloud", "AWS"),

    # Alibaba Cloud & Services
    (["aliyun", "aliyuncs"], "Cloud", "Alibaba Cloud"),
    (["alipay", "alipaydns", "alipayobjects"], "Payment", "Alipay"),
    (["alicdn", "alikunlun"], "CDN", "Alibaba CDN"),

    # Baidu Services
    (["baidu.com"], "Search", "Baidu"),
    (["bdstatic.com", "bdydns.com", "bdbus"], "CDN", "Baidu CDN"),
    (["shifen.com"], "CDN", "Baidu CDN (Shifen)"),

    # ByteDance
    (["byteoversea.com", "bytedance"], "CDN", "ByteDance"),
    (["byteglb.com"], "CDN", "ByteDance Global CDN"),

    # CDN & Load Balancer
    (["mgslb.com"], "CDN", "Load Balancer/CDN"),
    (["lbaas", "lb-"], "Network", "Load Balancer"),
    (["kunlunhuf", "lahuashanbx", "kunlunle", "kunluncan"], "CDN", "China CDN (Kunlun)"),
    (["quic.cloud"], "CDN", "QUIC.cloud"),
    (["fontawesome.com"], "CDN", "Font Awesome"),
    (["cdn", "cloudflare", "akamai", "fastly"], "CDN", "CDN"),

    # Network Infrastructure
    (["one.one.one.one"], "Network", "Cloudflare DNS (1.1.1.1)"),
    (["ipv4only.arpa"], "Network", "IPv4 Test"),
    (["in-addr.arpa"], "Network", "Reverse DNS"),
    (["dnspod.cn"], "Network", "DNSPod"),
    (["arena.ne.jp"], "ISP", "NTT Arena"),
    (["ylive.jp"], "ISP", "Yahoo Live"),
    (["landscape.co.jp"], "Network", "Landscape"),

    # Security
    (["mcafee.com"], "Security", "McAfee"),
    (["hcaptcha.com"], "Security", "hCaptcha"),
    (["cybertrust.ne.jp"], "Security", "Cybertrust"),
    (["waf"], "Security", "WAF"),

    # Payment
    (["stripe.com", "stripe.network"], "Payment", "Stripe"),
    (["americanexpress.com", "aexp-static.com"], "Payment", "American Express"),
    (["paypal"], "Payment", "PayPal"),
    (["payhub.jp"], "Payment", "PayHub"),

    # Microsoft
    (["msidentity.com", "msa.msidentity"], "Microsoft", "Microsoft Identity"),
    (["sharepoint.com"], "Microsoft", "SharePoint"),
    (["1drv.com", "onedrive"], "Microsoft", "OneDrive"),
    (["live.net"], "Microsoft", "Microsoft Live"),
    (["microsoft"], "Microsoft", "Microsoft"),

    # Social Media & Communication
    (["t.co"], "Social", "Twitter (X)"),
    (["fbpigeon.com"], "Social", "Facebook"),
    (["sinaimg.cn"], "Social", "Sina Weibo"),
    (["ameblo.jp"], "Blog", "Ameba Blog"),
    (["twitter", "facebook", "instagram"], "Social", "Social Media"),

    # Development Tools & Software
    (["vscode-cdn.net"], "Development", "VS Code"),
    (["obsidian.md"], "Development", "Obsidian"),
    (["github"], "Development", "GitHub"),
    (["growthbook.io"], "Development", "GrowthBook"),
    (["openlitespeed.org"], "Development", "OpenLiteSpeed"),
    (["clip-studio.com"], "Software", "Clip Studio"),
    (["amd.com"], "Software", "AMD"),

    # Apps & Services
    (["starbucks.co.jp"], "Retail", "Starbucks"),
    (["uber.com"], "Transport", "Uber"),
    (["goodnotes"], "Productivity", "GoodNotes"),
    (["mapbox.com"], "Maps", "Mapbox"),
    (["mousegestures"], "Browser", "Mouse Gestures"),

    # Xiaomi / MIUI
    (["miui.com", "xiaomi"], "Xiaomi", "Xiaomi/MIUI"),

    # News & Media
    (["newspicks.com"], "News", "NewsPicks"),
    (["hjholdings.tv"], "Media", "HJ Holdings"),
    (["vidaahub.com"], "Media", "Vidaa Hub"),
    (["wetvinfo.com"], "Media", "WeTV"),

    # Hosting & Infrastructure
    (["vultrusercontent.com"], "Hosting", "Vultr"),
    (["lolipop.jp"], "Hosting", "Lolipop"),

    # E-commerce & Business
    (["szlcsc.com"], "Shopping", "LCSC Electronics"),
    (["accelatech.com"], "Business", "Accelatech"),
    (["listdl.com"], "Download", "List Download"),
    (["amazon", "rakuten", "yahoo"], "Shopping", "E-commerce"),

    # Cloud Functions
    (["cloudfunctions.net"], "Cloud", "Google Cloud Functions"),
    (["force.com"], "Cloud", "Salesforce"),

    # Messaging & Communication
    (["im-apps.net"], "Messaging", "Messaging Service"),
    (["sms-forwarder.com"], "Messaging", "SMS Forwarder"),
    (["onezapp.com"], "Messaging", "OneZapp"),

    # Testing & Development
    (["but.jp"], "Testing", "Testing Domain"),
    (["wujisite.com"], "Development", "Wuji Site"),
    (["tdos.vip"], "Development", "TDOS"),

    # ISP & Telecom
    (["ntt", "softbank", "docomo"], "Telecom", "Telecom"),
]

def classify_domain(domain: str) -> Tuple[str, str]:
    """
    ドメインをカテゴリとサービス名に分類

    Returns:
        (category, service) のタプル
    """
    domain_lower = domain.lower()

    # IPアドレス判定（xxx.xxx.xxx.xxx形式）
    import re
    if re.match(r'^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$', domain):
        return ("Network", "IP Address")

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
