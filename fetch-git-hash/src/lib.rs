extern crate proc_macro;
use proc_macro::TokenStream;

use std::process::Command;

fn get_hash() -> Option<String> {
    let stdout = Command::new("git")
        .args(&["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()?
        .stdout;
    let hash = String::from_utf8(stdout).ok()?.trim().to_owned();
    Some(hash)
}

fn is_clean() -> Option<bool> {
    Some(
        Command::new("git")
            .args(&["diff-index", "--quiet", "HEAD"])
            .output()
            .ok()?
            .status
            .success(),
    )
}

#[proc_macro]
pub fn fetch_git_hash(_item: TokenStream) -> TokenStream {
    let dirty = if is_clean().unwrap_or(true) {
        ""
    } else {
        "-dirty"
    };
    format!("\"{}{}\"", get_hash().unwrap_or("unknown".into()), dirty)
        .parse()
        .unwrap()
}
