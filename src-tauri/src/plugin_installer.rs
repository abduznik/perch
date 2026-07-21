use std::fs;
use std::path::PathBuf;

/// Plugin content — embedded at compile time from plugins/perch-bridge.js
const PLUGIN_CONTENT: &str = include_str!("../../plugins/perch-bridge.js");

/// Plugin version — bump this when the plugin API changes
const PLUGIN_VERSION: &str = "0.2.0";

/// Marker at the top of the plugin file to detect version
const VERSION_MARKER: &str = "// Version: ";

/// Get the opencode config directory (~/.config/opencode/)
fn opencode_config_dir() -> Option<PathBuf> {
    dirs().map(|d| d.join("opencode"))
}

/// Get the user's config directory cross-platform
fn dirs() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".config"));
    }
    None
}

/// Install the perch-bridge plugin to opencode's plugin directory
/// and register it in opencode.jsonc.
pub fn install_plugin() {
    let config_dir = match opencode_config_dir() {
        Some(d) => d,
        None => {
            log::warn!("Cannot determine opencode config directory, skipping plugin install");
            return;
        }
    };

    let plugin_dir = config_dir.join("plugin");
    let plugin_path = plugin_dir.join("perch-bridge.js");

    // Check if plugin already exists
    if plugin_path.exists() {
        if let Ok(existing) = fs::read_to_string(&plugin_path) {
            if let Some(version) = extract_version(&existing) {
                if *version >= *PLUGIN_VERSION {
                    log::info!("Plugin already installed (v{}), skipping", version);
                    // Still ensure it's registered in config
                    ensure_plugin_registered(&config_dir);
                    return;
                }
                log::info!("Updating plugin from v{} to v{}", version, PLUGIN_VERSION);
            } else {
                log::info!("Existing plugin has no version marker, overwriting");
            }
        }
    } else {
        log::info!("First run — installing perch-bridge plugin");
    }

    // Create the plugin directory if needed
    if let Err(e) = fs::create_dir_all(&plugin_dir) {
        log::error!("Failed to create plugin directory {:?}: {}", plugin_dir, e);
        return;
    }

    // Write the plugin file
    match fs::write(&plugin_path, PLUGIN_CONTENT) {
        Ok(()) => {
            log::info!("Plugin installed to {:?}", plugin_path);
        }
        Err(e) => {
            log::error!("Failed to write plugin to {:?}: {}", plugin_path, e);
            return;
        }
    }

    // Register plugin in opencode.jsonc
    ensure_plugin_registered(&config_dir);
}

/// Ensure the plugin is registered in opencode.jsonc
fn ensure_plugin_registered(config_dir: &std::path::Path) {
    let config_path = config_dir.join("opencode.jsonc");

    // Read existing config or create minimal one
    let mut config: serde_json::Value = if config_path.exists() {
        fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({ "$schema": "https://opencode.ai/config.json" }))
    } else {
        serde_json::json!({ "$schema": "https://opencode.ai/config.json" })
    };

    // Check if plugin is already registered
    let plugin_entry = "./plugin/perch-bridge.js";
    let plugins = config
        .as_object_mut()
        .and_then(|o| o.get("plugin"))
        .and_then(|p| p.as_array());

    let already_registered = plugins
        .map(|arr| {
            arr.iter()
                .any(|v| v.as_str().map(|s| s == plugin_entry).unwrap_or(false))
        })
        .unwrap_or(false);

    if already_registered {
        log::info!("Plugin already registered in opencode.jsonc");
        return;
    }

    // Add plugin to config
    let plugin_array = config
        .as_object_mut()
        .unwrap()
        .entry("plugin")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .unwrap();

    plugin_array.push(serde_json::json!(plugin_entry));

    // Write back
    let pretty = serde_json::to_string_pretty(&config).unwrap_or_default();
    match fs::write(&config_path, format!("{}\n", pretty)) {
        Ok(()) => {
            log::info!("Plugin registered in {:?}", config_path);
        }
        Err(e) => {
            log::error!("Failed to write config to {:?}: {}", config_path, e);
        }
    }
}

/// Extract version string from plugin content
fn extract_version(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(VERSION_MARKER) {
            return Some(rest.trim().to_string());
        }
    }
    None
}
