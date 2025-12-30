"""
LacisIdGenerator - lacisId生成器
ESP32 LacisIDGeneratorのPython移植版

lacisId = "3" + productType(3桁) + MAC12HEX + productCode(4桁)
合計20文字固定
"""

import logging
import subprocess
from typing import Optional

logger = logging.getLogger(__name__)


class LacisIdGenerator:
    """
    lacisId生成器（deviceWithMAC仕様）
    - MACは平文12桁HEXをそのまま埋め込む（数値変換禁止）
    - lacisId = "3" + productType(3桁) + MAC12HEX + productCode(4桁)
    - 合計20文字固定
    """

    # is21 のデフォルト設定
    DEFAULT_PRODUCT_TYPE = "021"  # is21
    DEFAULT_PRODUCT_CODE = "0001"  # 最初のデバイス

    def __init__(self, product_type: str = DEFAULT_PRODUCT_TYPE, product_code: str = DEFAULT_PRODUCT_CODE):
        """
        Args:
            product_type: 3桁の製品タイプ（例："021"）
            product_code: 4桁の製品コード（例："0001"）
        """
        self.product_type = product_type.zfill(3)[:3]  # 3桁に正規化
        self.product_code = product_code.zfill(4)[:4]  # 4桁に正規化
        self._mac_cache: Optional[str] = None

    def generate(self, product_type: Optional[str] = None, product_code: Optional[str] = None) -> str:
        """
        lacisIdを生成

        Args:
            product_type: 3桁の製品タイプ（省略時はデフォルト）
            product_code: 4桁の製品コード（省略時はデフォルト）

        Returns:
            lacisId（20文字）
        """
        pt = (product_type or self.product_type).zfill(3)[:3]
        pc = (product_code or self.product_code).zfill(4)[:4]
        mac = self.get_primary_mac_hex()

        lacis_id = f"3{pt}{mac}{pc}"

        if len(lacis_id) != 20:
            logger.warning(f"[LACIS] Invalid lacisId length: {len(lacis_id)}, id: {lacis_id}")

        return lacis_id

    def get_primary_mac_hex(self) -> str:
        """
        プライマリNICのMACアドレスを12桁HEX（大文字、コロン無し）で取得

        優先順位:
        1. eth0（有線）
        2. enp* / end*（有線、命名規則）
        3. wlan0（無線）
        4. 最初に見つかったMAC

        Returns:
            MAC12HEX（例："AABBCCDDEEFF"）
        """
        if self._mac_cache:
            return self._mac_cache

        mac = self._get_mac_from_interface("eth0")
        if not mac:
            mac = self._get_mac_from_pattern("enp")
        if not mac:
            mac = self._get_mac_from_pattern("end")
        if not mac:
            mac = self._get_mac_from_interface("wlan0")
        if not mac:
            mac = self._get_first_mac()
        if not mac:
            logger.error("[LACIS] Could not get MAC address, using fallback")
            mac = "000000000000"

        self._mac_cache = mac
        return mac

    def _get_mac_from_interface(self, interface: str) -> Optional[str]:
        """特定インターフェースのMACを取得"""
        try:
            path = f"/sys/class/net/{interface}/address"
            with open(path, "r") as f:
                mac_raw = f.read().strip()
                mac = mac_raw.replace(":", "").upper()
                if len(mac) == 12 and mac != "000000000000":
                    logger.debug(f"[LACIS] MAC from {interface}: {mac}")
                    return mac
        except FileNotFoundError:
            pass
        except Exception as e:
            logger.debug(f"[LACIS] Error reading MAC from {interface}: {e}")
        return None

    def _get_mac_from_pattern(self, prefix: str) -> Optional[str]:
        """パターンマッチでインターフェースを検索"""
        try:
            import os
            net_dir = "/sys/class/net"
            if os.path.isdir(net_dir):
                for iface in os.listdir(net_dir):
                    if iface.startswith(prefix):
                        mac = self._get_mac_from_interface(iface)
                        if mac:
                            return mac
        except Exception as e:
            logger.debug(f"[LACIS] Error scanning interfaces with prefix {prefix}: {e}")
        return None

    def _get_first_mac(self) -> Optional[str]:
        """最初に見つかった有効なMACを取得"""
        try:
            import os
            net_dir = "/sys/class/net"
            if os.path.isdir(net_dir):
                for iface in os.listdir(net_dir):
                    if iface == "lo":  # ループバックは除外
                        continue
                    mac = self._get_mac_from_interface(iface)
                    if mac and mac != "000000000000":
                        return mac
        except Exception as e:
            logger.debug(f"[LACIS] Error getting first MAC: {e}")
        return None

    def get_mac_formatted(self) -> str:
        """
        MACアドレスをコロン区切りで取得

        Returns:
            例："AA:BB:CC:DD:EE:FF"
        """
        mac = self.get_primary_mac_hex()
        return ":".join([mac[i:i+2] for i in range(0, 12, 2)])

    @staticmethod
    def reconstruct_from_mac(product_type: int, mac_bytes: bytes, product_code: str = "0001") -> str:
        """
        MACバイト配列からlacisIdを再構成（受信側で使用）

        Args:
            product_type: 製品タイプ（数値）
            mac_bytes: MACアドレス（6バイト）
            product_code: 製品コード

        Returns:
            lacisId（20文字）
        """
        pt = str(product_type).zfill(3)[:3]
        mac_hex = mac_bytes.hex().upper()
        pc = product_code.zfill(4)[:4]
        return f"3{pt}{mac_hex}{pc}"
