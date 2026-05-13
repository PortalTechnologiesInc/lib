# `portal-macros`

**Tiny proc-macro helpers** used by daemons and apps to stamp binaries with source identity.

| Macro | Behavior |
|-------|----------|
| `fetch_git_hash!` | Expands to a string: `git rev-parse --short=8 HEAD`, plus `-dirty` if the tree is not clean. Falls back to `PORTAL_GIT_HASH` env, then `"unknown"`. |

```rust
use portal_macros::fetch_git_hash;

const GIT: &str = fetch_git_hash!();
```

**License:** GPL-3.0-or-later (see `Cargo.toml`). Downstream crates must respect that when linking this proc-macro crate.
