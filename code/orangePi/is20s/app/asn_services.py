"""
ASN to Service Mapping
IPからサービスを判定するためのASN（自治システム番号）マッピング

Sources:
- https://ipinfo.io/
- https://bgp.he.net/
- https://www.peeringdb.com/
"""

# ASN → (サービス名, カテゴリ)
ASN_SERVICE_MAP = {
    # ========== 動画配信 ==========
    # Google/YouTube
    15169: ("YouTube/Google", "video"),
    36040: ("YouTube/Google", "video"),  # Google Global Cache
    36383: ("Google", "video"),
    36384: ("Google", "video"),
    36411: ("Google", "video"),
    36492: ("Google", "video"),
    43515: ("Google", "video"),  # Google Ireland
    396982: ("Google", "video"),

    # Netflix
    2906: ("Netflix", "video"),
    40027: ("Netflix", "video"),  # Netflix transit

    # Amazon Prime Video (shares with AWS)
    16509: ("Amazon/AWS", "cloud"),
    14618: ("Amazon/AWS", "cloud"),

    # Disney+/Hulu (uses AWS/Akamai mostly)

    # ========== SNS ==========
    # Meta (Facebook/Instagram/WhatsApp/Threads)
    32934: ("Meta/Instagram", "sns"),

    # TikTok/ByteDance
    138699: ("TikTok", "sns"),  # Singapore
    396986: ("TikTok", "sns"),  # US

    # Twitter/X
    13414: ("Twitter/X", "sns"),

    # Snap
    2044: ("Snapchat", "sns"),

    # Pinterest
    54113: ("Pinterest", "sns"),  # also Fastly

    # LinkedIn (Microsoft)
    14413: ("LinkedIn", "sns"),

    # ========== メッセンジャー ==========
    # LINE (uses various CDNs, Verda/Akamai)
    38631: ("LINE", "messenger"),  # LINE Corp

    # Discord
    49544: ("Discord", "messenger"),

    # Telegram
    62041: ("Telegram", "messenger"),
    59930: ("Telegram", "messenger"),

    # ========== クラウドストレージ ==========
    # Apple/iCloud
    714: ("Apple/iCloud", "cloud"),
    6185: ("Apple/iCloud", "cloud"),  # Apple CDN

    # Microsoft/OneDrive/Azure
    8075: ("Microsoft/Azure", "cloud"),

    # Dropbox
    19679: ("Dropbox", "cloud"),

    # Google Drive (same as YouTube ASNs)

    # ========== EC・ショッピング ==========
    # 楽天
    17676: ("楽天", "ec"),

    # ========== ゲーム ==========
    # Steam/Valve
    32590: ("Steam", "game"),

    # Sony/PlayStation
    # Note: Sony uses various transit providers

    # Nintendo
    394406: ("Nintendo", "game"),

    # Epic Games
    49340: ("Epic Games", "game"),

    # Riot Games
    6507: ("Riot Games", "game"),

    # Blizzard
    57976: ("Blizzard", "game"),

    # NetEase Games
    45062: ("NetEase", "game"),

    # ========== CDN・インフラ ==========
    # Cloudflare
    13335: ("Cloudflare", "cdn"),

    # Akamai
    20940: ("Akamai", "cdn"),
    16625: ("Akamai", "cdn"),

    # Fastly
    54113: ("Fastly", "cdn"),

    # ========== 中国系サービス ==========
    # Tencent (騰訊/微信/QQ)
    45090: ("Tencent/微信", "china"),  # Domestic
    132203: ("Tencent/微信", "china"),  # Global

    # Alibaba (阿里巴巴/淘宝/支付宝)
    45102: ("Alibaba/淘宝", "china"),
    37963: ("Alibaba/淘宝", "china"),

    # Baidu (百度)
    55967: ("Baidu/百度", "china"),

    # Sina/Weibo (新浪/微博)
    37936: ("Weibo/微博", "china"),

    # JD.com (京東) - uses AS137693 and others
    137693: ("JD/京東", "china"),

    # Huawei (華為)
    55990: ("Huawei/華為", "china"),
    136907: ("Huawei Cloud", "china"),

    # Xiaomi (小米)
    59019: ("Xiaomi/小米", "china"),

    # Bilibili (哔哩哔哩)
    137718: ("Bilibili", "china"),

    # Kuaishou (快手) - TODO: verify ASN

    # ========== 中国ゲーム ==========
    # miHoYo/HoYoverse (原神/崩壊) - uses AWS/Cloudflare mostly
    # NetEase (see above)

    # ========== 通信キャリア（参考） ==========
    # China Telecom
    4134: ("China Telecom", "carrier"),
    4809: ("China Telecom", "carrier"),

    # China Unicom
    4837: ("China Unicom", "carrier"),

    # China Mobile
    9808: ("China Mobile", "carrier"),
    56040: ("China Mobile", "carrier"),

    # NTT
    2914: ("NTT", "carrier"),

    # KDDI
    2516: ("KDDI", "carrier"),

    # SoftBank
    17676: ("SoftBank", "carrier"),  # Note: same as Rakuten in some contexts
}

# カテゴリ別の色・優先度
CATEGORY_INFO = {
    "video": {"label": "動画", "color": "#dc2626", "priority": 1},
    "sns": {"label": "SNS", "color": "#2563eb", "priority": 2},
    "messenger": {"label": "メッセ", "color": "#7c3aed", "priority": 3},
    "game": {"label": "ゲーム", "color": "#059669", "priority": 4},
    "china": {"label": "中国系", "color": "#ea580c", "priority": 5},
    "ec": {"label": "EC", "color": "#0891b2", "priority": 6},
    "cloud": {"label": "クラウド", "color": "#6b7280", "priority": 7},
    "cdn": {"label": "CDN", "color": "#9ca3af", "priority": 8},
    "carrier": {"label": "通信", "color": "#d1d5db", "priority": 9},
}


def get_service_by_asn(asn: int) -> tuple:
    """
    ASN番号からサービス情報を取得
    Returns: (service_name, category) or (None, None)
    """
    if asn in ASN_SERVICE_MAP:
        return ASN_SERVICE_MAP[asn]
    return (None, None)


__all__ = ["ASN_SERVICE_MAP", "CATEGORY_INFO", "get_service_by_asn"]
