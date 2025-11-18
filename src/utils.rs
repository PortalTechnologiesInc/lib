
use nostr::{key::PublicKey, nips::nip05::{Nip05Address, Nip05Profile}};
use rand::Rng;

pub fn random_string(lenght: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(lenght)
        .map(char::from)
        .collect()
}

pub async fn verify_nip05(nip05: &str, main_key: &PublicKey) -> bool {
    let address = match Nip05Address::parse(nip05) {
        Ok(address) => address,
        Err(_) => return false,
    };

    let url = address.url();
    let req = match reqwest::get(url.to_string()).await {
        Ok(req) => req,
        Err(_) => return false,
    };
    let nip05: serde_json::Value = match req.json().await {
        Ok(nip05) => nip05,
        Err(_) => return false,
    };

    nostr::nips::nip05::verify_from_json(&main_key, &address, &nip05)
}


pub async fn fetch_nip05_profile(nip05: &str) -> anyhow::Result<Nip05Profile> {

    let address = Nip05Address::parse(nip05)?;

    let url = address.url();
    let req = reqwest::get(url.to_string()).await?;
    let json = req.json().await?;

    let nip05 = Nip05Profile::from_json(&address, &json)?;
    Ok(nip05)
}