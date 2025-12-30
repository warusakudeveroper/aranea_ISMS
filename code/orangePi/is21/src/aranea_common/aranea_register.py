"""
AraneaRegister - araneaDeviceGate登録
ESP32 AraneaRegisterのPython移植版

デバイス登録フロー:
1. TenantPrimaryAuth（テナント管理者認証）を設定
2. registerDevice() でaraneaDeviceGateに登録
3. CIC・stateEndpoint・mqttEndpointを取得・保存
"""

import os
import json
import logging
import httpx
from dataclasses import dataclass
from typing import Optional
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class TenantPrimaryAuth:
    """テナント管理者認証（lacisOath）"""
    lacis_id: str  # テナントプライマリのlacisId
    user_id: str   # テナントプライマリのemail
    cic: str       # テナントプライマリのCIC


@dataclass
class RegisterResult:
    """登録結果"""
    ok: bool = False
    cic_code: str = ""
    state_endpoint: str = ""
    mqtt_endpoint: str = ""
    error: str = ""


class AraneaRegister:
    """
    araneaDeviceGate登録管理

    機能:
    - デバイス登録（HTTP POST）
    - CIC・エンドポイント永続化
    - 登録状態管理
    """

    # デフォルトのGate URL（本番環境）
    DEFAULT_GATE_URL = "https://us-central1-metatron-one.cloudfunctions.net/araneaDeviceGate"

    # 設定ファイルのキー
    KEY_CIC = "cic_code"
    KEY_STATE_ENDPOINT = "state_endpoint"
    KEY_MQTT_ENDPOINT = "mqtt_endpoint"
    KEY_REGISTERED = "registered"
    KEY_LACIS_ID = "lacis_id"
    KEY_TID = "tid"

    def __init__(self, config_dir: str = "/opt/is21/config"):
        self.gate_url = self.DEFAULT_GATE_URL
        self.tenant_auth: Optional[TenantPrimaryAuth] = None
        self.config_dir = Path(config_dir)
        self.config_file = self.config_dir / "aranea_registration.json"
        self._data: dict = {}
        self._load()

    def _load(self) -> None:
        """設定ファイルを読み込む"""
        try:
            if self.config_file.exists():
                with open(self.config_file, "r", encoding="utf-8") as f:
                    self._data = json.load(f)
                logger.debug(f"[ARANEA] Loaded registration data from {self.config_file}")
        except Exception as e:
            logger.error(f"[ARANEA] Failed to load registration data: {e}")
            self._data = {}

    def _save(self) -> None:
        """設定ファイルに保存"""
        try:
            self.config_dir.mkdir(parents=True, exist_ok=True)
            with open(self.config_file, "w", encoding="utf-8") as f:
                json.dump(self._data, f, indent=2)
            logger.debug(f"[ARANEA] Saved registration data to {self.config_file}")
        except Exception as e:
            logger.error(f"[ARANEA] Failed to save registration data: {e}")

    def begin(self, gate_url: Optional[str] = None) -> None:
        """
        初期化

        Args:
            gate_url: araneaDeviceGate URL（省略時はデフォルト）
        """
        if gate_url:
            self.gate_url = gate_url
        logger.info(f"[ARANEA] Initialized with gate URL: {self.gate_url}")

    def set_tenant_primary(self, auth: TenantPrimaryAuth) -> None:
        """
        テナント管理者認証を設定

        Args:
            auth: TenantPrimaryAuth
        """
        self.tenant_auth = auth
        logger.info(f"[ARANEA] Tenant primary set: {auth.user_id}")

    def register_device(
        self,
        tid: str,
        device_type: str,
        lacis_id: str,
        mac_address: str,
        product_type: str,
        product_code: str,
        timeout: float = 15.0
    ) -> RegisterResult:
        """
        デバイスをaraneaDeviceGateに登録

        Args:
            tid: テナントID
            device_type: デバイスタイプ（例："ar-is21"）
            lacis_id: このデバイスのlacisId
            mac_address: MACアドレス（12桁HEX）
            product_type: 製品タイプ（3桁）
            product_code: 製品コード（4桁）
            timeout: HTTPタイムアウト（秒）

        Returns:
            RegisterResult
        """
        result = RegisterResult()

        # 既に登録済みの場合は保存データを返す
        if self.is_registered():
            saved_cic = self.get_saved_cic()
            if saved_cic:
                result.ok = True
                result.cic_code = saved_cic
                result.state_endpoint = self.get_saved_state_endpoint()
                result.mqtt_endpoint = self.get_saved_mqtt_endpoint()
                logger.info(f"[ARANEA] Already registered, using saved CIC: {saved_cic}")
                return result
            else:
                # CICが空の場合は再登録
                logger.warning("[ARANEA] Registered flag set but CIC is empty, re-registering...")
                self.clear_registration()

        # TenantPrimaryAuthが設定されていない場合はエラー
        if not self.tenant_auth:
            result.error = "TenantPrimaryAuth not set"
            logger.error(f"[ARANEA] {result.error}")
            return result

        # JSON構築
        payload = {
            "lacisOath": {
                "lacisId": self.tenant_auth.lacis_id,
                "userId": self.tenant_auth.user_id,
                "cic": self.tenant_auth.cic,
                "method": "register"
            },
            "userObject": {
                "lacisID": lacis_id,
                "tid": tid,
                "typeDomain": "araneaDevice",
                "type": device_type
            },
            "deviceMeta": {
                "macAddress": mac_address,
                "productType": product_type,
                "productCode": product_code
            }
        }

        logger.info("[ARANEA] Registering device...")
        logger.debug(f"[ARANEA] URL: {self.gate_url}")
        logger.debug(f"[ARANEA] Payload: {json.dumps(payload)}")

        # HTTP POST
        try:
            with httpx.Client(timeout=timeout) as client:
                response = client.post(
                    self.gate_url,
                    json=payload,
                    headers={"Content-Type": "application/json"}
                )

            logger.info(f"[ARANEA] Response code: {response.status_code}")
            logger.debug(f"[ARANEA] Response body: {response.text}")

            if response.status_code in (200, 201):
                data = response.json()
                if data.get("ok"):
                    result.ok = True
                    result.cic_code = data.get("userObject", {}).get("cic_code", "")
                    result.state_endpoint = data.get("stateEndpoint", "")
                    result.mqtt_endpoint = data.get("mqttEndpoint", "")

                    # 保存
                    self._data[self.KEY_CIC] = result.cic_code
                    self._data[self.KEY_STATE_ENDPOINT] = result.state_endpoint
                    self._data[self.KEY_MQTT_ENDPOINT] = result.mqtt_endpoint
                    self._data[self.KEY_REGISTERED] = True
                    self._data[self.KEY_LACIS_ID] = lacis_id
                    self._data[self.KEY_TID] = tid
                    self._save()

                    logger.info(f"[ARANEA] Registered! CIC: {result.cic_code}")
                    logger.info(f"[ARANEA] State endpoint: {result.state_endpoint}")
                    if result.mqtt_endpoint:
                        logger.info(f"[ARANEA] MQTT endpoint: {result.mqtt_endpoint}")
                else:
                    result.error = data.get("error", "Unknown error")
                    logger.error(f"[ARANEA] Registration failed: {result.error}")

            elif response.status_code == 409:
                result.error = "Device already registered (409)"
                logger.warning("[ARANEA] Device already registered (conflict)")

            else:
                try:
                    data = response.json()
                    result.error = data.get("error", f"HTTP {response.status_code}")
                except:
                    result.error = f"HTTP {response.status_code}"
                logger.error(f"[ARANEA] Error: {result.error}")

        except httpx.TimeoutException:
            result.error = "Connection timeout"
            logger.error(f"[ARANEA] {result.error}")
        except Exception as e:
            result.error = str(e)
            logger.error(f"[ARANEA] HTTP error: {e}")

        return result

    def is_registered(self) -> bool:
        """登録済みかどうか"""
        return self._data.get(self.KEY_REGISTERED, False)

    def get_saved_cic(self) -> str:
        """保存されたCICを取得"""
        return self._data.get(self.KEY_CIC, "")

    def get_saved_state_endpoint(self) -> str:
        """保存されたstateEndpointを取得"""
        return self._data.get(self.KEY_STATE_ENDPOINT, "")

    def get_saved_mqtt_endpoint(self) -> str:
        """保存されたmqttEndpointを取得"""
        return self._data.get(self.KEY_MQTT_ENDPOINT, "")

    def get_saved_lacis_id(self) -> str:
        """保存されたlacisIdを取得"""
        return self._data.get(self.KEY_LACIS_ID, "")

    def get_saved_tid(self) -> str:
        """保存されたtidを取得"""
        return self._data.get(self.KEY_TID, "")

    def clear_registration(self) -> None:
        """登録データをクリア"""
        self._data = {}
        self._save()
        logger.info("[ARANEA] Registration cleared")

    def get_registration_info(self) -> dict:
        """登録情報を辞書で取得"""
        return {
            "registered": self.is_registered(),
            "cic_code": self.get_saved_cic(),
            "lacis_id": self.get_saved_lacis_id(),
            "tid": self.get_saved_tid(),
            "state_endpoint": self.get_saved_state_endpoint(),
            "mqtt_endpoint": self.get_saved_mqtt_endpoint(),
        }
