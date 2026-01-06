/// Extract XAddr from capability section
pub fn extract_capability_xaddr(xml: &str, capability: &str) -> Option<String> {
    // Look for pattern like <Analytics><XAddr>http://...</XAddr>
    let cap_patterns = [
        format!("<{}>", capability),
        format!(":{}>", capability),
    ];

    for pattern in &cap_patterns {
        if let Some(cap_start) = xml.find(pattern.as_str()) {
            let cap_section = &xml[cap_start..];
            let end_pattern = format!("</{}", capability);
            if let Some(end_idx) = cap_section.find(end_pattern.as_str()) {
                let section = &cap_section[..end_idx];
                return extract_xml_value(section, "XAddr");
            }
        }
    }
    None
}

/// Extract XML attribute value
pub fn extract_xml_attribute(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let patterns = [
        format!("<{}{}=", tag, attr),
        format!(":{} {}=", tag, attr),
    ];

    for pattern in &patterns {
        if let Some(start) = xml.find(pattern.as_str()) {
            let after = &xml[start + pattern.len()..];
            // Find quote type
            let quote = if after.starts_with('"') { '"' } else { '\'' };
            if let Some(val_start) = after.find(quote) {
                let val_content = &after[val_start + 1..];
                if let Some(val_end) = val_content.find(quote) {
                    return Some(val_content[..val_end].to_string());
                }
            }
        }
    }

    // Try more flexible pattern
    let tag_patterns = [
        format!("<{}", tag),
        format!(":{}", tag),
    ];

    for pattern in &tag_patterns {
        if let Some(tag_start) = xml.find(pattern.as_str()) {
            let after_tag = &xml[tag_start..];
            if let Some(tag_end) = after_tag.find('>') {
                let tag_content = &after_tag[..tag_end];
                let attr_pattern = format!("{}=", attr);
                if let Some(attr_start) = tag_content.find(attr_pattern.as_str()) {
                    let after_attr = &tag_content[attr_start + attr_pattern.len()..];
                    let quote = if after_attr.starts_with('"') { '"' } else { '\'' };
                    if let Some(val_start) = after_attr.find(quote) {
                        let val_content = &after_attr[val_start + 1..];
                        if let Some(val_end) = val_content.find(quote) {
                            return Some(val_content[..val_end].to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract XML value with namespace-agnostic matching
pub fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    // Try with common ONVIF namespace prefix first
    let prefixed_patterns = [
        format!("<tds:{}>", tag),
        format!("<tt:{}>", tag),
        format!("<:{}>", tag),
    ];

    for pattern in &prefixed_patterns {
        if let Some(start) = xml.find(pattern.as_str()) {
            let content_start = start + pattern.len();
            if let Some(end) = xml[content_start..].find("</") {
                let value = xml[content_start..content_start + end].trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }

    // Try without namespace prefix
    let pattern = format!(":{}>", tag);
    if let Some(start) = xml.find(pattern.as_str()) {
        let content_start = start + pattern.len();
        if let Some(end) = xml[content_start..].find("</") {
            let value = xml[content_start..content_start + end].trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    // Final fallback: any tag ending with the name
    let simple_pattern = format!("<{}>", tag);
    if let Some(start) = xml.find(simple_pattern.as_str()) {
        let content_start = start + simple_pattern.len();
        let close_pattern = format!("</{}>", tag);
        if let Some(end) = xml[content_start..].find(close_pattern.as_str()) {
            let value = xml[content_start..content_start + end].trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}
