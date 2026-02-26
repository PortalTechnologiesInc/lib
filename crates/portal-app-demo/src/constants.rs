//! Paths for portal-app-demo config and data under the user's home.

use std::path::PathBuf;

/// Directory name for portal-app-demo config and data under the user's home.
const PORTAL_APP_DEMO_DIR: &str = ".portal-app-demo";

/// Returns the portal-app-demo directory path (e.g. `~/.portal-app-demo`).
pub fn portal_app_demo_dir() -> anyhow::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Home directory not found"))
        .map(|h| h.join(PORTAL_APP_DEMO_DIR))
}

/// Returns the Breez storage directory under portal-app-demo (e.g. `~/.portal-app-demo/breez`).
pub fn breez_storage_dir() -> anyhow::Result<PathBuf> {
    portal_app_demo_dir().map(|d| d.join("breez"))
}
