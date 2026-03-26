pub mod conversation;
#[cfg(feature = "profile-service")]
pub mod profile_service;
pub mod protocol;
pub mod router;
pub mod utils;

#[cfg(feature = "profile-service")]
pub use profile_service::register_nip05;

pub use nostr;
pub use nostr_relay_pool;

#[cfg(feature = "bindings")]
uniffi::setup_scaffolding!();

#[cfg(test)]
mod test_framework;
