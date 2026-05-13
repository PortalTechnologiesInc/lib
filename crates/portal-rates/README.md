# `portal-rates`

**BTC priced in fiat** using the same multi-provider aggregation style as BlueWallet—ported to Rust for daemons, mobile bindings, and anything else in this workspace that needs a rate.

```rust
use portal_rates::{MarketAPI, RatesError};

#[tokio::main]
async fn main() -> Result<(), RatesError> {
    let api = MarketAPI::new()?;
    let data = api.fetch_market_data("EUR").await?;

    println!("BTC/EUR: {}", data.price);
    println!("1000 EUR = {:.8} BTC", data.calculate_btc(1000.0));
    println!("1000 EUR = {} sats", data.calculate_sats(1000.0));

    Ok(())
}
```

| Topic | Detail |
|-------|--------|
| Providers | CoinGecko, Coinbase, Kraken, Bitstamp, Yadio, Coinpaprika, Exir, BNR, CoinDesk, … |
| Currencies | 50+ fiat tickers (USD, EUR, GBP, JPY, …) |
| Feature `bindings` | Enables UniFFI scaffolding for non-Rust callers |

```toml
[dependencies]
portal-rates = { path = "../portal-rates", features = ["bindings"] }
```

In Rust code the crate name uses an underscore: `portal_rates`.
