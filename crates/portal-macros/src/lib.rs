extern crate proc_macro;

use proc_macro::{Literal, TokenStream, TokenTree};
use std::process::Command;

// Git

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
    let from_git_command = get_hash().map(|hash| format!("{}{}", hash, dirty));
    let env = std::env::var("PORTAL_GIT_HASH").ok();

    let commit = from_git_command.or(env).unwrap_or("unknown".into());
    
    // format!("\"{}\"", commit).parse().unwrap()
    TokenStream::from(TokenTree::Literal(Literal::string(&commit)))
}