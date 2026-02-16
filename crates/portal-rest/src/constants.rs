use std::path::PathBuf;

/// Directory name for portal-rest config and data under the user's home.
const PORTAL_REST_DIR: &str = ".portal-rest";

/// Returns the portal-rest directory path (e.g. `~/.portal-rest`).
pub fn portal_rest_dir() -> anyhow::Result<PathBuf> {
    dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Home directory not found"))
        .map(|h| h.join(PORTAL_REST_DIR))
}
