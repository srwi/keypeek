use serde_json::Value;

#[cfg(feature = "hidapi")]
use super::json_parser::parse_layout_json_str;
#[cfg(any(feature = "hidapi", feature = "ureq"))]
use crate::protocols::DeviceError;
#[cfg(feature = "hidapi")]
use keypeek_core::KeyboardDefinition;

pub const VIA_CATALOG_INDEX_URL: &str = "https://usevia.app/definitions/supported_kbs.json";

#[inline]
pub fn via_vendor_product_id(vid: u16, pid: u16) -> u64 {
    ((vid as u64) << 16) | (pid as u64)
}

#[inline]
pub fn cache_stem(vid: u16, pid: u16) -> String {
    format!("{vid:04X}_{pid:04X}")
}

pub fn resolve_vendor_product_ids(json: &Value, target_vid: u16, target_pid: u16) -> (u16, u16) {
    // When connected to a physical device, the hardware USB IDs take precedence
    // over any vendorId/productId metadata embedded within the layout JSON.
    if target_vid != 0 || target_pid != 0 {
        return (target_vid, target_pid);
    }

    let vid = json
        .get("vendorId")
        .and_then(Value::as_str)
        .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .or_else(|| {
            json.get("vendorProductId")
                .and_then(Value::as_u64)
                .map(|id| (id >> 16) as u16)
        })
        .unwrap_or(0);

    let pid = json
        .get("productId")
        .and_then(Value::as_str)
        .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .or_else(|| {
            json.get("vendorProductId")
                .and_then(Value::as_u64)
                .map(|id| (id & 0xffff) as u16)
        })
        .unwrap_or(0);

    (vid, pid)
}

pub fn find_definition_version(index: &Value, vid: u16, pid: u16) -> Option<&'static str> {
    let vpid = via_vendor_product_id(vid, pid);
    let ids = index.get("vendorProductIds")?;
    if ids
        .get("v3")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|id| id.as_u64() == Some(vpid))
    {
        Some("v3")
    } else if ids
        .get("v2")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|id| id.as_u64() == Some(vpid))
    {
        Some("v2")
    } else {
        None
    }
}

pub fn definition_url(version: &str, vid: u16, pid: u16) -> String {
    format!(
        "https://usevia.app/definitions/{version}/{}.json",
        via_vendor_product_id(vid, pid)
    )
}

#[cfg(feature = "directories")]
pub fn get_desktop_cached_layout(vid: u16, pid: u16) -> Option<String> {
    let dirs = directories::ProjectDirs::from("dev", "srwi", "KeyPeek")?;
    let path = dirs
        .cache_dir()
        .join("via_layouts")
        .join(format!("{}.json", cache_stem(vid, pid)));
    std::fs::read_to_string(path).ok()
}

#[cfg(feature = "directories")]
pub fn save_desktop_cached_layout(vid: u16, pid: u16, content: &str) {
    if let Some(dirs) = directories::ProjectDirs::from("dev", "srwi", "KeyPeek") {
        let dir = dirs.cache_dir().join("via_layouts");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", cache_stem(vid, pid)));
        let _ = std::fs::write(path, content);
    }
}

#[cfg(feature = "ureq")]
fn fetch_url_text(url: &str) -> Result<String, DeviceError> {
    ureq::get(url)
        .call()
        .map_err(|e| DeviceError::Transport(format!("Failed to fetch '{url}': {e}")))?
        .into_body()
        .read_to_string()
        .map_err(|e| DeviceError::Transport(format!("Failed to read response from '{url}': {e}")))
}

#[cfg(feature = "ureq")]
pub fn fetch_desktop_catalog_layout(vid: u16, pid: u16) -> Result<Option<String>, DeviceError> {
    let index_text = fetch_url_text(VIA_CATALOG_INDEX_URL)?;
    let index: Value = serde_json::from_str(&index_text)
        .map_err(|e| DeviceError::Protocol(format!("Invalid VIA catalog JSON: {e}")))?;

    let Some(version) = find_definition_version(&index, vid, pid) else {
        return Ok(None);
    };

    let url = definition_url(version, vid, pid);
    let def_text = fetch_url_text(&url)?;
    Ok(Some(def_text))
}

#[cfg(feature = "hidapi")]
pub fn load_desktop_via_definition(
    vid: u16,
    pid: u16,
    json_path: &str,
) -> Result<KeyboardDefinition, DeviceError> {
    // 1. If user supplied an explicit path, load and parse that file
    let trimmed = json_path.trim();
    if !trimmed.is_empty() {
        let content = std::fs::read_to_string(trimmed).map_err(|e| {
            DeviceError::Protocol(format!("Failed to read layout file '{trimmed}': {e}"))
        })?;
        let parsed = parse_layout_json_str(&content, vid, pid)
            .map_err(|e| DeviceError::Protocol(format!("Failed to parse layout file: {e}")))?;
        #[cfg(feature = "directories")]
        save_desktop_cached_layout(vid, pid, &content);
        return Ok(parsed);
    }

    // 2. Check local disk cache
    #[cfg(feature = "directories")]
    if let Some(cached) = get_desktop_cached_layout(vid, pid) {
        match parse_layout_json_str(&cached, vid, pid) {
            Ok(parsed) => return Ok(parsed),
            Err(e) => {
                log::warn!("Cached desktop layout for {vid:04X}:{pid:04X} invalid: {e}");
            }
        }
    }

    // 3. Attempt to fetch from official VIA catalog
    #[cfg(feature = "ureq")]
    {
        log::info!("Checking official VIA catalog for {vid:04X}:{pid:04X}...");
        match fetch_desktop_catalog_layout(vid, pid) {
            Ok(Some(content)) => match parse_layout_json_str(&content, vid, pid) {
                Ok(parsed) => {
                    log::info!(
                        "Successfully loaded VIA layout from catalog for {vid:04X}:{pid:04X}"
                    );
                    #[cfg(feature = "directories")]
                    save_desktop_cached_layout(vid, pid, &content);
                    return Ok(parsed);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse VIA catalog definition for {vid:04X}:{pid:04X}: {e}"
                    );
                }
            },
            Ok(None) => {
                log::info!("Device {vid:04X}:{pid:04X} not found in official VIA catalog");
            }
            Err(e) => {
                log::warn!("Failed to fetch official VIA catalog: {e}");
            }
        }
    }

    // 4. Fall back to prompting user for layout file
    Err(DeviceError::LayoutFileRequired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_via_vendor_product_id() {
        assert_eq!(via_vendor_product_id(0x1234, 0x5678), 0x12345678);
        assert_eq!(via_vendor_product_id(0x0001, 0x0002), 0x00010002);
    }

    #[test]
    fn test_find_definition_version() {
        let index = json!({
            "vendorProductIds": {
                "v3": [0x12345678u64],
                "v2": [0xAAAABBBBu64]
            }
        });

        assert_eq!(find_definition_version(&index, 0x1234, 0x5678), Some("v3"));
        assert_eq!(find_definition_version(&index, 0xAAAA, 0xBBBB), Some("v2"));
        assert_eq!(find_definition_version(&index, 0x9999, 0x9999), None);
    }

    #[test]
    fn test_definition_url() {
        assert_eq!(
            definition_url("v3", 0x1234, 0x5678),
            "https://usevia.app/definitions/v3/305419896.json"
        );
    }

    #[test]
    fn test_cache_stem() {
        assert_eq!(cache_stem(0x1234, 0x5678), "1234_5678");
        assert_eq!(cache_stem(0x000A, 0x000B), "000A_000B");
    }

    #[test]
    fn test_resolve_vendor_product_ids() {
        let from_hex = json!({ "vendorId": "0x1234", "productId": "0x5678" });
        assert_eq!(
            resolve_vendor_product_ids(&from_hex, 0, 0),
            (0x1234, 0x5678)
        );

        let from_vpid = json!({ "vendorProductId": 0x12345678u64 });
        assert_eq!(
            resolve_vendor_product_ids(&from_vpid, 0, 0),
            (0x1234, 0x5678)
        );

        assert_eq!(
            resolve_vendor_product_ids(&from_hex, 0xAAAA, 0xBBBB),
            (0xAAAA, 0xBBBB)
        );

        let fallback = json!({});
        assert_eq!(
            resolve_vendor_product_ids(&fallback, 0xAAAA, 0xBBBB),
            (0xAAAA, 0xBBBB)
        );
    }
}
