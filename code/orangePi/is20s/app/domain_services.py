"""
Domain to Service Mapping
ドメインパターンからサービス名・カテゴリを判定

Features:
- JSONファイルで永続化 (/var/lib/is20s/domain_services.json)
- メモリにロードして高速検索
- 変更時のみファイル書き込み（SDカード保護）
- CacheSyncManagerと連携してピア間同期
"""
import json
import logging
from collections import OrderedDict
from datetime import datetime
from pathlib import Path
from threading import Lock
from typing import Any, Dict, List, Optional, Tuple

logger = logging.getLogger(__name__)


class UnknownDomainsCache:
    """
    未検出ドメインのFIFOキャッシュ

    特徴:
    - 最大300件保持（超えたら古いものから削除）
    - ドメインごとに出現回数・最終検出時刻を記録
    - API経由で未検出一覧を取得可能
    """

    MAX_SIZE = 300

    def __init__(self):
        self._lock = Lock()
        # OrderedDict: domain -> {count, first_seen, last_seen, sample_ips}
        self._cache: OrderedDict[str, Dict[str, Any]] = OrderedDict()

    def add(self, domain: str, src_ip: str = None, room_no: str = None) -> None:
        """未検出ドメインを追加"""
        if not domain:
            return

        now = datetime.now().isoformat()

        with self._lock:
            if domain in self._cache:
                # 既存: カウント増加、最終検出時刻更新
                entry = self._cache[domain]
                entry["count"] += 1
                entry["last_seen"] = now
                if src_ip and src_ip not in entry["sample_ips"]:
                    if len(entry["sample_ips"]) < 5:
                        entry["sample_ips"].append(src_ip)
                if room_no and room_no not in entry["rooms"]:
                    entry["rooms"].append(room_no)
                # 最近使用したものを末尾に移動
                self._cache.move_to_end(domain)
            else:
                # 新規追加
                self._cache[domain] = {
                    "count": 1,
                    "first_seen": now,
                    "last_seen": now,
                    "sample_ips": [src_ip] if src_ip else [],
                    "rooms": [room_no] if room_no else [],
                }
                # 最大サイズ超過時は古いものを削除
                while len(self._cache) > self.MAX_SIZE:
                    self._cache.popitem(last=False)

    def get_all(self) -> List[Dict[str, Any]]:
        """全未検出ドメインを取得（カウント降順）"""
        with self._lock:
            result = []
            for domain, info in self._cache.items():
                result.append({
                    "domain": domain,
                    "count": info["count"],
                    "first_seen": info["first_seen"],
                    "last_seen": info["last_seen"],
                    "sample_ips": info["sample_ips"],
                    "rooms": info["rooms"],
                })
            # カウント降順でソート
            result.sort(key=lambda x: -x["count"])
            return result

    def get_count(self) -> int:
        """未検出ドメイン数を取得"""
        with self._lock:
            return len(self._cache)

    def get_total_hits(self) -> int:
        """総ヒット数を取得"""
        with self._lock:
            return sum(info["count"] for info in self._cache.values())

    def clear(self) -> None:
        """キャッシュをクリア"""
        with self._lock:
            self._cache.clear()

    def remove(self, domain: str) -> bool:
        """特定のドメインを削除"""
        with self._lock:
            if domain in self._cache:
                del self._cache[domain]
                return True
            return False


# 未検出キャッシュシングルトン
_unknown_cache: Optional[UnknownDomainsCache] = None


def get_unknown_cache() -> UnknownDomainsCache:
    """UnknownDomainsCacheシングルトンを取得"""
    global _unknown_cache
    if _unknown_cache is None:
        _unknown_cache = UnknownDomainsCache()
    return _unknown_cache


def record_unknown_domain(domain: str, src_ip: str = None, room_no: str = None) -> None:
    """未検出ドメインを記録（便利関数）"""
    get_unknown_cache().add(domain, src_ip, room_no)

# データファイルパス
DATA_DIR = Path("/var/lib/is20s")
DOMAIN_SERVICES_FILE = DATA_DIR / "domain_services.json"

# デフォルトサービスマップ（ui.pyから移行）
# 形式: pattern -> {"service": "...", "category": "..."}
DEFAULT_DOMAIN_SERVICES: Dict[str, Dict[str, str]] = {
    # 動画・配信
    "youtube": {"service": "YouTube", "category": "Streaming"},
    "ytimg": {"service": "YouTube", "category": "Streaming"},
    "yt3.ggpht": {"service": "YouTube", "category": "Streaming"},
    "googlevideo": {"service": "YouTube", "category": "Streaming"},
    "youtubei": {"service": "YouTube", "category": "Streaming"},
    "netflix": {"service": "Netflix", "category": "Streaming"},
    "nflxvideo": {"service": "Netflix", "category": "Streaming"},
    "nflximg": {"service": "Netflix", "category": "Streaming"},
    "nflxext": {"service": "Netflix", "category": "Streaming"},
    "nicovideo": {"service": "ニコニコ", "category": "Streaming"},
    "nimg": {"service": "ニコニコ", "category": "Streaming"},
    "niconi.co": {"service": "ニコニコ", "category": "Streaming"},
    "twitchcdn": {"service": "Twitch", "category": "Streaming"},
    "twitch.tv": {"service": "Twitch", "category": "Streaming"},
    "jtvnw": {"service": "Twitch", "category": "Streaming"},
    "abema": {"service": "ABEMA", "category": "Streaming"},
    "ameba": {"service": "Ameba", "category": "SNS"},
    "hulu": {"service": "Hulu", "category": "Streaming"},
    "huluim": {"service": "Hulu", "category": "Streaming"},
    "dazn": {"service": "DAZN", "category": "Streaming"},
    "primevideo": {"service": "Prime Video", "category": "Streaming"},
    "aiv-cdn": {"service": "Prime Video", "category": "Streaming"},
    "disneyplus": {"service": "Disney+", "category": "Streaming"},
    "spotify": {"service": "Spotify", "category": "Streaming"},
    "scdn": {"service": "Spotify", "category": "Streaming"},
    "applemusic": {"service": "Apple Music", "category": "Streaming"},
    "soundcloud": {"service": "SoundCloud", "category": "Streaming"},
    # SNS
    "instagram": {"service": "Instagram", "category": "SNS"},
    "cdninstagram": {"service": "Instagram", "category": "SNS"},
    "facebook": {"service": "Facebook", "category": "SNS"},
    "fbcdn": {"service": "Facebook", "category": "SNS"},
    "fb.com": {"service": "Facebook", "category": "SNS"},
    "twitter": {"service": "X", "category": "SNS"},
    "x.com": {"service": "X", "category": "SNS"},
    "twimg": {"service": "X", "category": "SNS"},
    "threads.net": {"service": "Threads", "category": "SNS"},
    "tiktok": {"service": "TikTok", "category": "SNS"},
    "bytedance": {"service": "TikTok", "category": "SNS"},
    "tiktokcdn": {"service": "TikTok", "category": "SNS"},
    "musical.ly": {"service": "TikTok", "category": "SNS"},
    "pinterest": {"service": "Pinterest", "category": "SNS"},
    "pinimg": {"service": "Pinterest", "category": "SNS"},
    "linkedin": {"service": "LinkedIn", "category": "SNS"},
    "snapchat": {"service": "Snapchat", "category": "SNS"},
    "snapkit": {"service": "Snapchat", "category": "SNS"},
    "bereal": {"service": "BeReal", "category": "SNS"},
    # 漫画・エンタメ
    "piccoma": {"service": "ピッコマ", "category": "Entertainment"},
    "kakaocdn": {"service": "Kakao", "category": "Entertainment"},
    # メッセンジャー
    "line": {"service": "LINE", "category": "Messenger"},
    "linecorp": {"service": "LINE", "category": "Messenger"},
    "line-apps": {"service": "LINE", "category": "Messenger"},
    "line-scdn": {"service": "LINE", "category": "Messenger"},
    "naver": {"service": "LINE", "category": "Messenger"},
    "wechat": {"service": "WeChat", "category": "Messenger"},
    "weixin": {"service": "WeChat", "category": "Messenger"},
    "qq.com": {"service": "QQ/WeChat", "category": "Messenger"},
    "tencent": {"service": "Tencent", "category": "Messenger"},
    "whatsapp": {"service": "WhatsApp", "category": "Messenger"},
    "telegram": {"service": "Telegram", "category": "Messenger"},
    "t.me": {"service": "Telegram", "category": "Messenger"},
    "discord": {"service": "Discord", "category": "Messenger"},
    "discordapp": {"service": "Discord", "category": "Messenger"},
    "slack": {"service": "Slack", "category": "Messenger"},
    "slack-edge": {"service": "Slack", "category": "Messenger"},
    "kakaotalk": {"service": "KakaoTalk", "category": "Messenger"},
    "kakao": {"service": "Kakao", "category": "Messenger"},
    "signal": {"service": "Signal", "category": "Messenger"},
    "viber": {"service": "Viber", "category": "Messenger"},
    # Google
    "googleusercontent": {"service": "GPhotos", "category": "Photo"},
    "photos.google": {"service": "GPhotos", "category": "Photo"},
    "googleapis": {"service": "Google", "category": "Cloud"},
    "gstatic": {"service": "Google", "category": "Cloud"},
    "ggpht": {"service": "Google", "category": "Cloud"},
    "googlesyndication": {"service": "GoogleAds", "category": "Ad"},
    "doubleclick": {"service": "GoogleAds", "category": "Ad"},
    "googleadservices": {"service": "GoogleAds", "category": "Ad"},
    "gvt1": {"service": "Google", "category": "Cloud"},
    "gvt2": {"service": "Google", "category": "Cloud"},
    "gmail": {"service": "Gmail", "category": "Mail"},
    "googlemail": {"service": "Gmail", "category": "Mail"},
    "drive.google": {"service": "GDrive", "category": "Cloud"},
    "docs.google": {"service": "GDocs", "category": "Cloud"},
    "meet.google": {"service": "GMeet", "category": "Video"},
    "google-analytics": {"service": "GA", "category": "Tracker"},
    "google": {"service": "Google", "category": "Cloud"},
    # Apple
    "itunes": {"service": "iTunes", "category": "Cloud"},
    "itunes-apple": {"service": "iTunes", "category": "Cloud"},
    "appstore": {"service": "AppStore", "category": "Cloud"},
    "apps.apple": {"service": "AppStore", "category": "Cloud"},
    "amp-api": {"service": "AppStore", "category": "Cloud"},
    "icloud": {"service": "iCloud", "category": "Photo"},
    "apple-cloudkit": {"service": "iCloud", "category": "Photo"},
    "apple.map": {"service": "Maps", "category": "Cloud"},
    "apple-map": {"service": "Maps", "category": "Cloud"},
    "ical": {"service": "iCal", "category": "Cloud"},
    "caldav": {"service": "iCal", "category": "Cloud"},
    "imessage": {"service": "iMessage", "category": "Messenger"},
    "push.apple": {"service": "Push", "category": "Cloud"},
    "mzstatic": {"service": "Apple", "category": "Cloud"},
    "apple": {"service": "Apple", "category": "Cloud"},
    "mac.com": {"service": "Apple", "category": "Cloud"},
    # CDN
    "fastly.net": {"service": "Fastly", "category": "CDN"},
    "akamai": {"service": "Akamai", "category": "CDN"},
    "edgekey": {"service": "Akamai", "category": "CDN"},
    "edgesuite": {"service": "Akamai", "category": "CDN"},
    "akadns": {"service": "Akamai", "category": "CDN"},
    "akamaized": {"service": "Akamai", "category": "CDN"},
    "cloudflare": {"service": "Cloudflare", "category": "CDN"},
    "cf-cdn": {"service": "Cloudflare", "category": "CDN"},
    "fastly": {"service": "Fastly", "category": "CDN"},
    "edgecast": {"service": "Edgecast", "category": "CDN"},
    "jsdelivr": {"service": "jsDelivr", "category": "CDN"},
    "unpkg": {"service": "unpkg", "category": "CDN"},
    # 日本メディア
    "tbs.co.jp": {"service": "TBS", "category": "Media"},
    "nhk.or.jp": {"service": "NHK", "category": "Media"},
    "ntv.co.jp": {"service": "日テレ", "category": "Media"},
    "fujitv": {"service": "フジ", "category": "Media"},
    "tv-asahi": {"service": "テレ朝", "category": "Media"},
    "tv-tokyo": {"service": "テレ東", "category": "Media"},
    "radiko": {"service": "radiko", "category": "Streaming"},
    # SDK
    "safedk": {"service": "SafeDK", "category": "SDK"},
    # Microsoft
    "microsoft": {"service": "Microsoft", "category": "Cloud"},
    "msft": {"service": "Microsoft", "category": "Cloud"},
    "azure": {"service": "Azure", "category": "Cloud"},
    "windows": {"service": "Windows", "category": "Cloud"},
    "msn": {"service": "MSN", "category": "Cloud"},
    "office": {"service": "Office365", "category": "Cloud"},
    "outlook": {"service": "Outlook", "category": "Mail"},
    "hotmail": {"service": "Outlook", "category": "Mail"},
    "live.com": {"service": "Microsoft", "category": "Cloud"},
    "onedrive": {"service": "OneDrive", "category": "Cloud"},
    "bing": {"service": "Bing", "category": "Cloud"},
    "skype": {"service": "Skype", "category": "Video"},
    "teams": {"service": "Teams", "category": "Video"},
    # EC
    "amazon": {"service": "Amazon", "category": "EC"},
    "cloudfront": {"service": "AWS/CDN", "category": "CDN"},
    "amazonaws": {"service": "AWS", "category": "Cloud"},
    "compute-1": {"service": "AWS", "category": "Cloud"},
    "ec2-": {"service": "AWS", "category": "Cloud"},
    "rakuten": {"service": "楽天", "category": "EC"},
    "yahoo": {"service": "Yahoo", "category": "EC"},
    "yimg": {"service": "Yahoo", "category": "EC"},
    "mercari": {"service": "メルカリ", "category": "EC"},
    "zozotown": {"service": "ZOZO", "category": "EC"},
    "zozo": {"service": "ZOZO", "category": "EC"},
    "paypay": {"service": "PayPay", "category": "EC"},
    "shopify": {"service": "Shopify", "category": "EC"},
    "aliexpress": {"service": "AliExpress", "category": "EC"},
    "alibaba": {"service": "Alibaba", "category": "EC"},
    "temu": {"service": "Temu", "category": "EC"},
    "shein": {"service": "SHEIN", "category": "EC"},
    # ゲーム
    "steam": {"service": "Steam", "category": "Game"},
    "steampowered": {"service": "Steam", "category": "Game"},
    "steamcommunity": {"service": "Steam", "category": "Game"},
    "playstation": {"service": "PlayStation", "category": "Game"},
    "sony": {"service": "Sony", "category": "Game"},
    "xbox": {"service": "Xbox", "category": "Game"},
    "nintendo": {"service": "Nintendo", "category": "Game"},
    "epicgames": {"service": "Epic Games", "category": "Game"},
    "unrealengine": {"service": "Epic", "category": "Game"},
    "riotgames": {"service": "Riot", "category": "Game"},
    "leagueoflegends": {"service": "LoL", "category": "Game"},
    "blizzard": {"service": "Blizzard", "category": "Game"},
    "battle.net": {"service": "Blizzard", "category": "Game"},
    "mihoyo": {"service": "miHoYo", "category": "Game"},
    "hoyoverse": {"service": "HoYoverse", "category": "Game"},
    "cygames": {"service": "Cygames", "category": "Game"},
    # クラウド・ストレージ
    "dropbox": {"service": "Dropbox", "category": "Cloud"},
    "box.com": {"service": "Box", "category": "Cloud"},
    "github": {"service": "GitHub", "category": "Cloud"},
    "githubusercontent": {"service": "GitHub", "category": "Cloud"},
    "githubassets": {"service": "GitHub", "category": "Cloud"},
    "gitlab": {"service": "GitLab", "category": "Cloud"},
    "bitbucket": {"service": "Bitbucket", "category": "Cloud"},
    "atlassian": {"service": "Atlassian", "category": "Cloud"},
    # ビデオ会議
    "zoom": {"service": "Zoom", "category": "Video"},
    "zoomcdn": {"service": "Zoom", "category": "Video"},
    "webex": {"service": "Webex", "category": "Video"},
    "cisco": {"service": "Cisco", "category": "Video"},
    # AI
    "chatgpt": {"service": "ChatGPT", "category": "AI"},
    "openai": {"service": "OpenAI", "category": "AI"},
    "anthropic": {"service": "Anthropic", "category": "AI"},
    "claude": {"service": "Claude", "category": "AI"},
    "statsig": {"service": "Analytics", "category": "Tracker"},
    "gemini": {"service": "Gemini", "category": "AI"},
    "bard": {"service": "Bard", "category": "AI"},
    "copilot": {"service": "Copilot", "category": "AI"},
    "cursor": {"service": "Cursor", "category": "AI"},
    "cursor.sh": {"service": "Cursor", "category": "AI"},
    # ニュース
    "nhk": {"service": "NHK", "category": "News"},
    "asahi": {"service": "朝日新聞", "category": "News"},
    "yomiuri": {"service": "読売新聞", "category": "News"},
    "nikkei": {"service": "日経", "category": "News"},
    "cnn": {"service": "CNN", "category": "News"},
    "bbc": {"service": "BBC", "category": "News"},
    # アダルト
    "pornhub": {"service": "PornHub", "category": "Adult"},
    "xvideos": {"service": "XVideos", "category": "Adult"},
    "xhamster": {"service": "xHamster", "category": "Adult"},
    "xnxx": {"service": "XNXX", "category": "Adult"},
    "cityheaven": {"service": "CityHeaven", "category": "Adult"},
    "fuzoku": {"service": "風俗", "category": "Adult"},
    # Tor
    "tor": {"service": "Tor", "category": "VPN"},
    "onion": {"service": "Tor", "category": "VPN"},
    # 広告・トラッキング
    "moloco": {"service": "Moloco", "category": "Ad"},
    "adsmoloco": {"service": "Moloco", "category": "Ad"},
    "applovin": {"service": "AppLovin", "category": "Ad"},
    "fivecdm": {"service": "AdSDK", "category": "Ad"},
    "adtng": {"service": "AdTech", "category": "Ad"},
    "adnxs": {"service": "AdTech", "category": "Ad"},
    "adsrvr": {"service": "AdTech", "category": "Ad"},
    "criteo": {"service": "Criteo", "category": "Ad"},
    "bance": {"service": "AdTech", "category": "Ad"},
    "adjust": {"service": "Adjust", "category": "Tracker"},
    "appsflyer": {"service": "AppsFlyer", "category": "Tracker"},
    "branch": {"service": "Branch", "category": "Tracker"},
    "amplitude": {"service": "Amplitude", "category": "Tracker"},
    "segment": {"service": "Segment", "category": "Tracker"},
    "mixpanel": {"service": "Mixpanel", "category": "Tracker"},
    "taboola": {"service": "Taboola", "category": "Ad"},
    "outbrain": {"service": "Outbrain", "category": "Ad"},
    "smartnews-ads": {"service": "SmartNews", "category": "Ad"},
    "pubmatic": {"service": "PubMatic", "category": "Ad"},
    "rubiconproject": {"service": "Rubicon", "category": "Ad"},
    "openx": {"service": "OpenX", "category": "Ad"},
    "casalemedia": {"service": "Casale", "category": "Ad"},
    "advertising": {"service": "AdTech", "category": "Ad"},
    "adsafeprotected": {"service": "AdSafe", "category": "Ad"},
    "reflected": {"service": "AdTech", "category": "Ad"},
    "wcdnga": {"service": "AdTech", "category": "Ad"},
    "trafficjunky": {"service": "AdTech", "category": "Ad"},
    "adhigh": {"service": "AdTech", "category": "Ad"},
    "adcolony": {"service": "AdColony", "category": "Ad"},
    "unity3d": {"service": "Unity", "category": "Ad"},
    "unityads": {"service": "UnityAds", "category": "Ad"},
    "chartboost": {"service": "Chartboost", "category": "Ad"},
    "vungle": {"service": "Vungle", "category": "Ad"},
    "ironsrc": {"service": "IronSource", "category": "Ad"},
    "fyber": {"service": "Fyber", "category": "Ad"},
    "inmobi": {"service": "InMobi", "category": "Ad"},
    "mopub": {"service": "MoPub", "category": "Ad"},
    "relaido": {"service": "Relaido", "category": "Ad"},
    "deepintent": {"service": "DeepIntent", "category": "Ad"},
    "admanmedia": {"service": "AdMan", "category": "Ad"},
    "turn.com": {"service": "Turn", "category": "Ad"},
    "media.net": {"service": "MediaNet", "category": "Ad"},
    "btloader": {"service": "BTLoader", "category": "Ad"},
    "trustarc": {"service": "TrustArc", "category": "Tracker"},
    "tsdtocl": {"service": "AdTech", "category": "Ad"},
    "profilepassport": {"service": "ProfilePassport", "category": "Tracker"},
    "netseer": {"service": "NetSeer", "category": "Ad"},
    "d-markets": {"service": "D-Markets", "category": "Ad"},
    "ytv-viewing": {"service": "YTV", "category": "Tracker"},
    "digicert": {"service": "DigiCert", "category": "Cert"},
    "ocsp": {"service": "OCSP", "category": "Cert"},
    "rustdesk": {"service": "RustDesk", "category": "Remote"},
    # アナリティクス
    "hotjar": {"service": "Hotjar", "category": "Tracker"},
    "mouseflow": {"service": "Mouseflow", "category": "Tracker"},
    "fullstory": {"service": "FullStory", "category": "Tracker"},
    "crazyegg": {"service": "CrazyEgg", "category": "Tracker"},
    "heap": {"service": "Heap", "category": "Tracker"},
    "intercom": {"service": "Intercom", "category": "Tracker"},
    # VPN
    "openvpn": {"service": "OpenVPN", "category": "VPN"},
    "wireguard": {"service": "WireGuard", "category": "VPN"},
    "nordvpn": {"service": "NordVPN", "category": "VPN"},
    "expressvpn": {"service": "ExpressVPN", "category": "VPN"},
    "surfshark": {"service": "Surfshark", "category": "VPN"},
    "protonvpn": {"service": "ProtonVPN", "category": "VPN"},
    "ipvanish": {"service": "IPVanish", "category": "VPN"},
    "cyberghost": {"service": "CyberGhost", "category": "VPN"},
    "privateinternetaccess": {"service": "PIA", "category": "VPN"},
    "mullvad": {"service": "Mullvad", "category": "VPN"},
    "tunnelbear": {"service": "TunnelBear", "category": "VPN"},
    # その他
    "wordpress": {"service": "WordPress", "category": "Cloud"},
    "wix": {"service": "Wix", "category": "Cloud"},
    "cloudinary": {"service": "Cloudinary", "category": "CDN"},
    "imgix": {"service": "imgix", "category": "CDN"},
}


class DomainServiceManager:
    """
    ドメイン→サービスマッピング管理

    特徴:
    - メモリ上で検索（高速）
    - LRUキャッシュで繰り返しルックアップを高速化
    - 変更時のみファイル書き込み（SDカード保護）
    - スレッドセーフ
    """

    CACHE_MAX_SIZE = 10000  # LRUキャッシュ最大エントリ数

    def __init__(self, data_file: Path = DOMAIN_SERVICES_FILE):
        self.data_file = data_file
        self._lock = Lock()
        self._data: Dict[str, Dict[str, str]] = {}
        self._dirty = False
        # LRUキャッシュ: domain -> (service, category)
        self._cache: OrderedDict[str, Tuple[Optional[str], Optional[str]]] = OrderedDict()
        self._cache_hits = 0
        self._cache_misses = 0
        self._load()

    def _load(self) -> None:
        """JSONファイルからロード"""
        with self._lock:
            if self.data_file.exists():
                try:
                    with self.data_file.open() as f:
                        loaded = json.load(f)
                        # バージョン2: services直下にマップ
                        if isinstance(loaded, dict) and "services" in loaded:
                            self._data = loaded["services"]
                        else:
                            # 旧形式互換
                            self._data = loaded if isinstance(loaded, dict) else {}
                    logger.info("Loaded %d domain services from file", len(self._data))
                except Exception as e:
                    logger.warning("Failed to load domain services: %s", e)
                    self._data = {}

            # 空の場合はデフォルトを使用
            if not self._data:
                self._data = DEFAULT_DOMAIN_SERVICES.copy()
                self._dirty = True
                self._save_internal()
                logger.info("Initialized with %d default domain services", len(self._data))

    def _save_internal(self) -> bool:
        """内部保存（ロック取得済み前提）"""
        if not self._dirty:
            return True
        try:
            self.data_file.parent.mkdir(parents=True, exist_ok=True)
            data = {
                "version": 2,
                "services": self._data,
            }
            with self.data_file.open("w") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            self._dirty = False
            logger.info("Saved %d domain services", len(self._data))
            return True
        except Exception as e:
            logger.error("Failed to save domain services: %s", e)
            return False

    def save(self) -> bool:
        """明示的に保存"""
        with self._lock:
            return self._save_internal()

    def get_service(self, domain: str) -> Tuple[Optional[str], Optional[str]]:
        """
        ドメインからサービス名・カテゴリを取得（LRUキャッシュ対応）

        Args:
            domain: ドメイン名（例: "video.google.com"）

        Returns:
            (service_name, category) or (None, None)
        """
        if not domain:
            return (None, None)

        domain_lower = domain.lower()

        with self._lock:
            # キャッシュヒットチェック
            if domain_lower in self._cache:
                self._cache_hits += 1
                # LRU: 末尾に移動
                self._cache.move_to_end(domain_lower)
                return self._cache[domain_lower]

            self._cache_misses += 1

            # ドット区切りを考慮したマッチング
            result: Tuple[Optional[str], Optional[str]] = (None, None)
            for pattern, info in self._data.items():
                if "." in pattern:
                    # パターンにドットがある場合は厳密マッチ
                    if (
                        domain_lower == pattern
                        or domain_lower.endswith("." + pattern)
                        or ("." + pattern + ".") in domain_lower
                    ):
                        result = (info.get("service"), info.get("category"))
                        break
                else:
                    # パターンにドットがない場合は部分マッチ
                    if pattern in domain_lower:
                        result = (info.get("service"), info.get("category"))
                        break

            # キャッシュに保存
            self._cache[domain_lower] = result
            # 最大サイズ超過時は古いものを削除
            while len(self._cache) > self.CACHE_MAX_SIZE:
                self._cache.popitem(last=False)

            return result

    def _invalidate_cache(self) -> None:
        """キャッシュを無効化（データ変更時に呼び出し）"""
        self._cache.clear()
        logger.debug("Service lookup cache invalidated")

    def add(self, pattern: str, service: str, category: str = "") -> Dict[str, Any]:
        """サービスマッピングを追加"""
        if not pattern or not service:
            return {"ok": False, "error": "Pattern and service required"}

        pattern_lower = pattern.lower()
        with self._lock:
            self._data[pattern_lower] = {"service": service, "category": category}
            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Added: {pattern} -> {service}"}

    def update(
        self, pattern: str, service: str = None, category: str = None
    ) -> Dict[str, Any]:
        """サービスマッピングを更新"""
        pattern_lower = pattern.lower()
        with self._lock:
            if pattern_lower not in self._data:
                return {"ok": False, "error": f"Pattern not found: {pattern}"}

            if service is not None:
                self._data[pattern_lower]["service"] = service
            if category is not None:
                self._data[pattern_lower]["category"] = category

            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Updated: {pattern}"}

    def delete(self, pattern: str) -> Dict[str, Any]:
        """サービスマッピングを削除"""
        pattern_lower = pattern.lower()
        with self._lock:
            if pattern_lower not in self._data:
                return {"ok": False, "error": f"Pattern not found: {pattern}"}

            del self._data[pattern_lower]
            self._dirty = True
            self._invalidate_cache()
            self._save_internal()

        return {"ok": True, "message": f"Deleted: {pattern}"}

    def get_all(self) -> Dict[str, Dict[str, str]]:
        """全マッピングを取得"""
        with self._lock:
            return self._data.copy()

    def get_count(self) -> int:
        """登録数を取得"""
        with self._lock:
            return len(self._data)

    def search(self, query: str) -> List[Dict[str, Any]]:
        """パターン・サービス名で検索"""
        query_lower = query.lower()
        results = []
        with self._lock:
            for pattern, info in self._data.items():
                if (
                    query_lower in pattern
                    or query_lower in info.get("service", "").lower()
                    or query_lower in info.get("category", "").lower()
                ):
                    results.append(
                        {
                            "pattern": pattern,
                            "service": info.get("service"),
                            "category": info.get("category"),
                        }
                    )
        return results

    def export_data(self) -> Dict[str, Any]:
        """エクスポート用データを取得"""
        with self._lock:
            return {
                "version": 2,
                "services": self._data.copy(),
            }

    def import_data(self, data: Dict[str, Any], merge: bool = True) -> Dict[str, Any]:
        """
        データをインポート

        Args:
            data: インポートデータ
            merge: Trueなら既存とマージ、Falseなら上書き

        Returns:
            結果と統計
        """
        stats = {"added": 0, "updated": 0, "skipped": 0}

        if "services" not in data:
            return {"ok": False, "error": "Invalid data format", "stats": stats}

        with self._lock:
            if not merge:
                self._data = {}

            for pattern, info in data["services"].items():
                if not isinstance(info, dict):
                    stats["skipped"] += 1
                    continue

                pattern_lower = pattern.lower()
                if pattern_lower in self._data:
                    # 既存エントリの更新
                    self._data[pattern_lower].update(info)
                    stats["updated"] += 1
                else:
                    # 新規追加
                    self._data[pattern_lower] = info
                    stats["added"] += 1

            self._dirty = True
            self._save_internal()

        return {"ok": True, "stats": stats}


# シングルトンインスタンス
_domain_service_manager: Optional[DomainServiceManager] = None


def get_domain_service_manager() -> DomainServiceManager:
    """DomainServiceManagerシングルトンを取得"""
    global _domain_service_manager
    if _domain_service_manager is None:
        _domain_service_manager = DomainServiceManager()
    return _domain_service_manager


def get_service_by_domain(domain: str) -> Tuple[Optional[str], Optional[str]]:
    """
    ドメインからサービス情報を取得（便利関数）

    Returns: (service_name, category) or (None, None)
    """
    return get_domain_service_manager().get_service(domain)


__all__ = [
    "DomainServiceManager",
    "get_domain_service_manager",
    "get_service_by_domain",
    "DOMAIN_SERVICES_FILE",
    "UnknownDomainsCache",
    "get_unknown_cache",
    "record_unknown_domain",
]
