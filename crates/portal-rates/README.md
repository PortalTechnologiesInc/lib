# Rates

Bitcoin exchange rates from multiple sources. Based on BlueWallet's logic.

## Usage

```rust
use rates::{MarketAPI, RatesError};

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

## Sources

- CoinGecko, Coinbase, Kraken, Bitstamp
- Yadio, Coinpaprika, Exir, BNR, CoinDesk

Supports 50+ currencies including USD, EUR, GBP, JPY, CAD, AUD.

## Features

- Fetch real-time BTC rates
- Calculate BTC, sats, and millisats from fiat amounts
- Optional UniFFI bindings for cross-platform use
- Detailed error handling for different failure scenarios

```toml
[dependencies]
rates = { version = "0.1.0", features = ["bindings"] }
```