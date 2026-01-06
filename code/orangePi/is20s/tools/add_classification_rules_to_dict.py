#!/usr/bin/env python3
"""
分類ルールから辞書パターン追加
classify_unknown_domains.py の CLASSIFICATION_RULES を default_domain_services.json に反映
"""
import json
import sys
from pathlib import Path

# 分類ルール（classify_unknown_domains.py から抽出）
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

    # AI/ML
    (["gliacloud"], "AI", "GliaCloud (AI Video)"),

    # Monitoring & Observability
    (["nr-data.net", "newrelic"], "Monitoring", "New Relic"),
    (["sentry.io"], "Monitoring", "Sentry"),
    (["honeycomb.io"], "Monitoring", "Honeycomb"),
    (["datadog", "datadoghq"], "Monitoring", "Datadog"),
    (["usonar.jp", "sonar"], "Monitoring", "U-SONAR Analytics"),

    # Tracking & Analytics
    (["keepa.com"], "Tracker", "Keepa (Price Tracker)"),
    (["app-measurement.com"], "Analytics", "Google Analytics"),
    (["googletagmanager.com"], "Analytics", "Google Tag Manager"),
    (["omtrdc.net", "omtr"], "Analytics", "Adobe Analytics"),
    (["imrworldwide.com", "comscore"], "Analytics", "Nielsen/comScore"),
    (["mieru-ca.com"], "Analytics", "Mieru-CA Analytics"),
    (["leanplum.com"], "Analytics", "Leanplum"),
    (["braze.com", "braze.eu"], "Analytics", "Braze"),
    (["appcenter.ms"], "Analytics", "App Center Analytics"),
    (["singular.net"], "Analytics", "Singular"),
    (["piyolog.com"], "Analytics", "Piyolog"),

    # Booking & Travel
    (["booking.com", "beds24.com"], "Booking", "Booking Service"),

    # IoT
    (["daikinsmartdb.jp", "daikin"], "IoT", "Daikin Smart"),
    (["tplinkra.com", "tplink"], "IoT", "TP-Link Router"),
    (["coolkit.cn", "coolkit.cc", "ewelink"], "IoT", "eWeLink IoT"),
    (["blynk.io", "blynk.cloud"], "IoT", "Blynk IoT"),
    (["home.mi.com"], "IoT", "Xiaomi Home"),

    # Ad Network
    (["exp-tas.com", "exponential"], "Ad", "Exponential (Ad Network)"),
    (["youborafds", "youboranqs"], "Ad", "Youbora Ad Network"),
    (["creativecdn.com"], "Ad", "Creative CDN"),
    (["uncn.jp"], "Ad", "Ad Network (Japan)"),
    (["a-dsp.com"], "Ad", "DSP"),
    (["nielsen.com"], "Ad", "Nielsen"),
    (["onelink.me"], "Ad", "AppsFlyer OneLink"),
    (["unrulymedia.com"], "Ad", "Unruly Media"),
    (["adingo.jp"], "Ad", "Adingo (Japan)"),
    (["id5-sync.com"], "Ad", "ID5"),
    (["wagbridge.com"], "CDN", "Wagbridge CDN"),

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
    (["kunlunhuf", "lahuashanbx", "kunlunle", "kunluncan"], "CDN", "China CDN (Kunlun)"),
    (["quic.cloud"], "CDN", "QUIC.cloud"),
    (["fontawesome.com"], "CDN", "Font Awesome"),

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

    # Payment
    (["stripe.com", "stripe.network"], "Payment", "Stripe"),
    (["americanexpress.com", "aexp-static.com"], "Payment", "American Express"),
    (["payhub.jp"], "Payment", "PayHub"),

    # Microsoft
    (["msidentity.com", "msa.msidentity"], "Microsoft", "Microsoft Identity"),
    (["sharepoint.com"], "Microsoft", "SharePoint"),
    (["1drv.com", "onedrive"], "Microsoft", "OneDrive"),
    (["live.net"], "Microsoft", "Microsoft Live"),

    # Xiaomi / MIUI
    (["mijia.tech", "io.mi.com"], "IoT", "Xiaomi Mi IoT"),
    (["miui.com", "xiaomi"], "Xiaomi", "Xiaomi/MIUI"),

    # Chinese Tech & Services
    (["kuaishou", "adukwai.com", "kwai", "yximgs.com", "bcelive.com"], "Social", "Kuaishou (快手)"),
    (["youku.tv", "youku.com"], "Streaming", "Youku (優酷)"),
    (["xiaojukeji.com", "didichuxing", "udache.com", "didistatic.com"], "Transport", "DiDi (滴滴)"),
    (["meituan.com", "meituan.net"], "Shopping", "Meituan (美団)"),
    (["dianping.com"], "Shopping", "Dianping (大衆点評)"),
    (["ele.me", "eleme"], "Shopping", "Ele.me (餓了麼)"),
    (["jd.com"], "Shopping", "JD.com (京東)"),
    (["iqiyi.com", "iq.com"], "Streaming", "iQiyi (愛奇芸)"),
    (["uc.cn"], "Browser", "UC Browser"),
    (["jpush.cn"], "Messaging", "JPush"),
    (["pcloud.tw"], "Cloud", "PCloud Taiwan"),
    (["tanx.com"], "Ad", "Taobao Ad Network"),

    # Southeast Asia Services
    (["grab.com", "grabtaxi.com"], "Transport", "Grab"),

    # E-commerce & Retail
    (["lkcoffee.com"], "Retail", "Luckin Coffee"),
    (["pchome.com.tw"], "Shopping", "PChome"),
    (["etsy.com"], "Shopping", "Etsy"),
    (["sekorm.com"], "Shopping", "Sekorm Electronics"),
    (["ameya360.com"], "Shopping", "Ameya360"),
    (["vipmro.net"], "Shopping", "Vipmro"),

    # Ad Tech (Additional)
    (["adseyeservice.com", "adseye.com"], "Ad", "AdSeye"),
    (["spadsync.com"], "Ad", "SpAdSync"),
    (["tongdun.net"], "Ad", "Tongdun"),

    # Analytics (Additional)
    (["reproio.com", "repro.io"], "Analytics", "Repro"),

    # Financial Services
    (["beaconbank.jp"], "Finance", "Beacon Bank"),
    (["openexchangerates.org"], "Finance", "Open Exchange Rates"),

    # Cloud Infrastructure (Additional)
    (["hwclouds-dns.com"], "Cloud", "Huawei Cloud"),
    (["jcloud-cdn.com", "jcloudgslb.com"], "CDN", "JCloud CDN"),

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

    # Mobile Apps & Services
    (["antutu.net", "ksmobile.com"], "Mobile", "AnTuTu Benchmark"),
]

def main():
    dict_file = Path(__file__).parent / "../data/default_domain_services.json"

    # 既存辞書読み込み
    with open(dict_file, 'r', encoding='utf-8') as f:
        dictionary = json.load(f)

    existing_patterns = set(dictionary['services'].keys())
    added_count = 0

    # 分類ルールから辞書に追加
    for keywords, category, service in CLASSIFICATION_RULES:
        for pattern in keywords:
            # 既に存在するパターンはスキップ
            if pattern in existing_patterns:
                continue

            # 辞書に追加
            dictionary['services'][pattern] = {
                "category": category,
                "service": service
            }
            added_count += 1
            print(f"  + {pattern:40s} [{category:10s}] {service}")

    # 保存
    with open(dict_file, 'w', encoding='utf-8') as f:
        json.dump(dictionary, f, indent=2, ensure_ascii=False)

    print(f"\n✓ 分類ルールから辞書更新完了")
    print(f"  既存: {len(existing_patterns)}パターン")
    print(f"  追加: {added_count}パターン")
    print(f"  合計: {len(dictionary['services'])}パターン")
    print(f"  保存先: {dict_file}")

if __name__ == "__main__":
    main()
