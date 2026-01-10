//! Camera LacisID Generation
//!
//! Phase 2: CameraRegistry (Issue #115)
//! DD05_CameraRegistry.md準拠
//!
//! ## LacisID形式 (カメラ用)
//! ```text
//! [Prefix=3][ProductType=801][MAC=12桁][ProductCode=4桁] = 20桁
//!            │                │         │
//!            │                │         └─ ブランド別コード
//!            │                └─ カメラのMACアドレス
//!            └─ is801 paraclateCamera
//! ```
//!
//! ## 例
//! MAC=AA:BB:CC:DD:EE:FF, ProductCode=0001 → LacisID=3801AABBCCDDEEFF0001

use super::types::{
    DEFAULT_PRODUCT_CODE, LACIS_ID_LENGTH, MAC_LENGTH, PREFIX, PRODUCT_CODE_LENGTH, PRODUCT_TYPE,
};

/// カメラのMACアドレスとProductCodeからLacisIDを生成
///
/// # Arguments
/// * `mac` - MACアドレス（任意形式: AA:BB:CC:DD:EE:FF, AA-BB-CC-DD-EE-FF, AABBCCDDEEFF）
/// * `product_code` - 4桁のProductCode（例: "0001" for Tapo）
///
/// # Returns
/// 20桁のLacisID文字列
///
/// # Panics
/// MACアドレスまたはProductCodeが不正な場合
pub fn generate_camera_lacis_id(mac: &str, product_code: &str) -> String {
    // MACアドレスからコロン/ハイフンを除去し大文字化
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    assert_eq!(
        mac_clean.len(),
        MAC_LENGTH,
        "Invalid MAC address length: expected {}, got {}",
        MAC_LENGTH,
        mac_clean.len()
    );

    // ProductCodeのパディング/トリミング
    let code = if product_code.len() >= PRODUCT_CODE_LENGTH {
        &product_code[..PRODUCT_CODE_LENGTH]
    } else {
        // 左0パディング
        &format!("{:0>4}", product_code)
    };

    format!("{}{}{}{}", PREFIX, PRODUCT_TYPE, mac_clean, code)
}

/// カメラのMACアドレスとProductCodeからLacisIDを生成（Result版）
///
/// # Arguments
/// * `mac` - MACアドレス
/// * `product_code` - ProductCode（省略時はデフォルト "0000"）
///
/// # Returns
/// Ok: 20桁のLacisID文字列
/// Err: MACアドレスが不正な場合
pub fn try_generate_camera_lacis_id(
    mac: &str,
    product_code: Option<&str>,
) -> Result<String, String> {
    let mac_clean: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    if mac_clean.len() != MAC_LENGTH {
        return Err(format!(
            "Invalid MAC address: expected {} hex digits, got {}",
            MAC_LENGTH,
            mac_clean.len()
        ));
    }

    let code = product_code.unwrap_or(DEFAULT_PRODUCT_CODE);
    let code_padded = if code.len() >= PRODUCT_CODE_LENGTH {
        code[..PRODUCT_CODE_LENGTH].to_string()
    } else {
        format!("{:0>4}", code)
    };

    let lacis_id = format!("{}{}{}{}", PREFIX, PRODUCT_TYPE, mac_clean, code_padded);

    // 最終長さチェック
    if lacis_id.len() != LACIS_ID_LENGTH {
        return Err(format!(
            "Generated LacisID has invalid length: expected {}, got {}",
            LACIS_ID_LENGTH,
            lacis_id.len()
        ));
    }

    Ok(lacis_id)
}

/// ブランド名からProductCodeを取得
pub fn get_product_code_for_brand(brand_name: &str) -> &'static str {
    let brand_lower = brand_name.to_lowercase();

    if brand_lower.contains("tapo") || brand_lower.contains("tp-link") {
        super::types::product_codes::TAPO
    } else if brand_lower.contains("hikvision") {
        super::types::product_codes::HIKVISION
    } else if brand_lower.contains("dahua") {
        super::types::product_codes::DAHUA
    } else if brand_lower.contains("reolink") {
        super::types::product_codes::REOLINK
    } else if brand_lower.contains("axis") {
        super::types::product_codes::AXIS
    } else {
        super::types::product_codes::GENERIC
    }
}

/// LacisIDをパース
pub fn parse_camera_lacis_id(lacis_id: &str) -> Result<ParsedLacisId, String> {
    if lacis_id.len() != LACIS_ID_LENGTH {
        return Err(format!(
            "Invalid LacisID length: expected {}, got {}",
            LACIS_ID_LENGTH,
            lacis_id.len()
        ));
    }

    let prefix = &lacis_id[0..1];
    let product_type = &lacis_id[1..4];
    let mac = &lacis_id[4..16];
    let product_code = &lacis_id[16..20];

    Ok(ParsedLacisId {
        prefix: prefix.to_string(),
        product_type: product_type.to_string(),
        mac: mac.to_string(),
        product_code: product_code.to_string(),
    })
}

/// パース済みLacisID
#[derive(Debug, Clone)]
pub struct ParsedLacisId {
    pub prefix: String,
    pub product_type: String,
    pub mac: String,
    pub product_code: String,
}

impl ParsedLacisId {
    /// MACアドレスをコロン区切りで取得
    pub fn mac_formatted(&self) -> String {
        let chars: Vec<char> = self.mac.chars().collect();
        chars
            .chunks(2)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(":")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_camera_lacis_id_basic() {
        assert_eq!(
            generate_camera_lacis_id("AA:BB:CC:DD:EE:FF", "0001"),
            "3801AABBCCDDEEFF0001"
        );
    }

    #[test]
    fn test_generate_camera_lacis_id_generic() {
        assert_eq!(
            generate_camera_lacis_id("AA:BB:CC:DD:EE:FF", "0000"),
            "3801AABBCCDDEEFF0000"
        );
    }

    #[test]
    fn test_generate_camera_lacis_id_length() {
        let lacis_id = generate_camera_lacis_id("AA:BB:CC:DD:EE:FF", "0002");
        assert_eq!(lacis_id.len(), 20);
    }

    #[test]
    fn test_try_generate_valid() {
        let result = try_generate_camera_lacis_id("AA:BB:CC:DD:EE:FF", Some("0001"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3801AABBCCDDEEFF0001");
    }

    #[test]
    fn test_try_generate_default_product_code() {
        let result = try_generate_camera_lacis_id("AA:BB:CC:DD:EE:FF", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3801AABBCCDDEEFF0000");
    }

    #[test]
    fn test_try_generate_invalid_mac() {
        let result = try_generate_camera_lacis_id("AA:BB:CC", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_product_code_for_brand() {
        assert_eq!(get_product_code_for_brand("Tapo"), "0001");
        assert_eq!(get_product_code_for_brand("TP-LINK"), "0001");
        assert_eq!(get_product_code_for_brand("Hikvision"), "0002");
        assert_eq!(get_product_code_for_brand("Unknown"), "0000");
    }

    #[test]
    fn test_parse_camera_lacis_id() {
        let result = parse_camera_lacis_id("3801AABBCCDDEEFF0001");
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.prefix, "3");
        assert_eq!(parsed.product_type, "801");
        assert_eq!(parsed.mac, "AABBCCDDEEFF");
        assert_eq!(parsed.product_code, "0001");
    }

    #[test]
    fn test_parsed_mac_formatted() {
        let parsed = parse_camera_lacis_id("3801AABBCCDDEEFF0001").unwrap();
        assert_eq!(parsed.mac_formatted(), "AA:BB:CC:DD:EE:FF");
    }
}
