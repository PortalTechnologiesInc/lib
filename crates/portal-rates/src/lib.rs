//! Implementation based on BlueWallet's fiat currency and market data logic,
//! ported to Rust. Original logic and fiat currency definitions are taken
//! from BlueWallet (https://github.com/BlueWallet/BlueWallet).

use core::fmt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Instant};
use thiserror::Error;

#[cfg(feature = "bindings")]
uniffi::setup_scaffolding!();

#[derive(Debug, Error)]
#[cfg_attr(feature = "bindings", derive(uniffi::Error))]
pub enum RatesError {
    #[error("Failed to parse JSON: {0}")]
    SerdeJsonError(String),

    #[error("HTTP request failed: {0}")]
    HttpRequest(String),

    #[error("Missing price in Yadio response")]
    MissingYadioPrice,

    #[error("Missing rate in YadioConvert response")]
    MissingYadioConvertRate,

    #[error("Missing CoinGecko price")]
    MissingCoinGeckoPrice,

    #[error("Missing 'last' price")]
    MissingLastPrice,

    #[error("Missing Coinpaprika INR price")]
    MissingCoinpaprikaInrPrice,

    #[error("Missing Coinbase amount")]
    MissingCoinbaseAmount,

    #[error("Missing Kraken close price")]
    MissingKrakenClosePrice,

    #[error("Missing CoinDesk field")]
    MissingCoinDeskField,

    #[error("Unsupported currency")]
    UnsupportedCurrency,

    #[error("Unsupported source or malformed JSON")]
    UnsupportedSource,

    #[error("Failed to fetch market data")]
    MarketDataFetchFailed,

    #[error("Failed to parse price as number: {0}")]
    PriceParseFailed(String),
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
struct FiatUnit {
    #[serde(rename = "endPointKey")]
    end_point_key: String,
    // locale: String,
    source: Source,
    symbol: String,
    // country: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bindings", derive(uniffi::Enum))]
enum Source {
    Yadio,
    YadioConvert,
    Exir,
    #[serde(rename = "coinpaprika")]
    Coinpaprika,
    Bitstamp,
    Coinbase,
    CoinGecko,
    BNR,
    Kraken,
    CoinDesk,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Yadio => write!(f, "yadio"),
            Source::YadioConvert => write!(f, "yadio_convert"),
            Source::Exir => write!(f, "exir"),
            Source::Coinpaprika => write!(f, "coinpaprika"),
            Source::Bitstamp => write!(f, "bitstamp"),
            Source::Coinbase => write!(f, "coinbase"),
            Source::CoinGecko => write!(f, "coingecko"),
            Source::BNR => write!(f, "bnr"),
            Source::Kraken => write!(f, "kraken"),
            Source::CoinDesk => write!(f, "coindesk"),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct MarketData {
    pub price: String,
    pub rate: f64,
    pub source: String,
}

impl MarketData {
    pub fn calculate_btc(&self, amount: f64) -> f64 {
        amount / self.rate
    }

    pub fn calculate_sats(&self, amount: f64) -> i64 {
        ((amount / self.rate) * 100_000_000.0) as i64
    }

    pub fn calculate_millisats(&self, amount: f64) -> i64 {
        ((amount / self.rate) * 100_000_000_000.0) as i64
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "bindings", derive(uniffi::Object))]
pub struct MarketAPI {
    fiat_units: HashMap<String, FiatUnit>,
    client: Client,
}

impl MarketAPI {
    fn build_url(source: &Source, key: &str) -> String {
        match source {
            Source::Yadio => format!("https://api.yadio.io/json/{}", key),
            Source::YadioConvert => format!("https://api.yadio.io/convert/1/BTC/{}", key),
            Source::Exir => "https://api.exir.io/v1/ticker?symbol=btc-irt".to_string(),
            Source::Coinpaprika => {
                "https://api.coinpaprika.com/v1/tickers/btc-bitcoin?quotes=INR".to_string()
            }
            Source::Bitstamp => {
                format!(
                    "https://www.bitstamp.net/api/v2/ticker/btc{}",
                    key.to_lowercase()
                )
            }
            Source::Coinbase => {
                format!(
                    "https://api.coinbase.com/v2/prices/BTC-{}/buy",
                    key.to_uppercase()
                )
            }
            Source::CoinGecko => format!(
                "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies={}",
                key.to_lowercase()
            ),
            Source::BNR => "https://www.bnr.ro/nbrfxrates.xml".to_string(),
            Source::Kraken => {
                format!(
                    "https://api.kraken.com/0/public/Ticker?pair=XXBTZ{}",
                    key.to_uppercase()
                )
            }
            Source::CoinDesk => {
                format!(
                    "https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms={}",
                    key.to_uppercase()
                )
            }
        }
    }

    fn parse_price_json(json: &str, source: &Source, key: &str) -> Result<String, RatesError> {
        let v: Value =
            serde_json::from_str(json).map_err(|e| RatesError::SerdeJsonError(e.to_string()))?;

        match source {
            Source::Yadio => {
                let price = v
                    .get(key)
                    .and_then(|obj| obj.get("price"))
                    .and_then(Value::as_str)
                    .ok_or(RatesError::MissingYadioPrice)?;
                Ok(price.to_string())
            }
            Source::YadioConvert => {
                let rate = v
                    .get("rate")
                    .and_then(Value::as_str)
                    .ok_or(RatesError::MissingYadioConvertRate)?;
                Ok(rate.to_string())
            }
            Source::CoinGecko => {
                let val = v
                    .get("bitcoin")
                    .and_then(|btc| btc.get(&key.to_lowercase()))
                    .ok_or(RatesError::MissingCoinGeckoPrice)?;
                Ok(val.to_string())
            }
            Source::Exir | Source::Bitstamp => {
                let val = v
                    .get("last")
                    .and_then(Value::as_str)
                    .ok_or(RatesError::MissingLastPrice)?;
                Ok(val.to_string())
            }
            Source::Coinpaprika => {
                let val = v
                    .get("quotes")
                    .and_then(|q| q.get("INR"))
                    .and_then(|inr| inr.get("price"))
                    .ok_or(RatesError::MissingCoinpaprikaInrPrice)?;
                Ok(val.to_string())
            }
            Source::Coinbase => {
                let val = v
                    .get("data")
                    .and_then(|d| d.get("amount"))
                    .and_then(Value::as_str)
                    .ok_or(RatesError::MissingCoinbaseAmount)?;
                Ok(val.to_string())
            }
            Source::Kraken => {
                let val = v
                    .get("result")
                    .and_then(|r| r.get(&format!("XXBTZ{}", key.to_uppercase())))
                    .and_then(|pair| pair.get("c"))
                    .and_then(|c| c.get(0))
                    .and_then(Value::as_str)
                    .ok_or(RatesError::MissingKrakenClosePrice)?;
                Ok(val.to_string())
            }
            Source::CoinDesk => {
                let val = v
                    .get(&key.to_uppercase())
                    .ok_or(RatesError::MissingCoinDeskField)?;
                Ok(val.to_string())
            }
            _ => Err(RatesError::UnsupportedSource),
        }
    }

    async fn fetch_price_for_source(
        self: Arc<Self>,
        source: &Source,
        key: &str,
    ) -> Result<Option<String>, RatesError> {
        let url = Self::build_url(source, key);
        let res = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RatesError::HttpRequest(e.to_string()))?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let text = res
            .text()
            .await
            .map_err(|e| RatesError::HttpRequest(e.to_string()))?;
        let parsed = Self::parse_price_json(&text, source, key)?;
        Ok(Some(parsed))
    }

    fn fallback_sources(primary: &Source) -> &'static [Source] {
        const KRAKEN_FALLBACKS: [Source; 2] = [Source::CoinGecko, Source::CoinDesk];
        const COINGECKO_FALLBACKS: [Source; 2] = [Source::Kraken, Source::CoinDesk];
        const YADIO_FALLBACKS: [Source; 2] = [Source::CoinGecko, Source::YadioConvert];
        const COINGECKO_ONLY_FALLBACKS: [Source; 1] = [Source::CoinGecko];

        match primary {
            Source::Kraken => &KRAKEN_FALLBACKS,
            Source::CoinGecko => &COINGECKO_FALLBACKS,
            Source::Yadio => &YADIO_FALLBACKS,
            Source::Exir => &COINGECKO_ONLY_FALLBACKS,
            Source::YadioConvert
            | Source::Coinpaprika
            | Source::Bitstamp
            | Source::Coinbase
            | Source::BNR
            | Source::CoinDesk => &COINGECKO_ONLY_FALLBACKS,
        }
    }

    async fn resolve_price_with_fallback(
        self: Arc<Self>,
        unit: &FiatUnit,
    ) -> Result<(String, Source), RatesError> {
        for source in std::iter::once(unit.source)
            .chain(Self::fallback_sources(&unit.source).iter().copied())
        {
            match self
                .clone()
                .fetch_price_for_source(&source, &unit.end_point_key)
                .await
            {
                Ok(Some(price_str)) => return Ok((price_str, source)),
                Ok(None) => {
                    log::debug!(
                        "No price from source={} for key={}",
                        source,
                        unit.end_point_key
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Price fetch failed from source={} for key={}: {}",
                        source,
                        unit.end_point_key,
                        e
                    );
                }
            }
        }

        Err(RatesError::MarketDataFetchFailed)
    }

    #[cfg(test)]
    async fn resolve_price_with_fallback_with_fetcher<F, Fut>(
        unit: &FiatUnit,
        mut fetcher: F,
    ) -> Result<(String, Source), RatesError>
    where
        F: FnMut(&Source, &str) -> Fut,
        Fut: std::future::Future<Output = Result<Option<String>, RatesError>>,
    {
        for source in std::iter::once(unit.source)
            .chain(Self::fallback_sources(&unit.source).iter().copied())
        {
            match fetcher(&source, &unit.end_point_key).await {
                Ok(Some(price_str)) => return Ok((price_str, source)),
                Ok(None) => {}
                Err(_) => {}
            }
        }

        Err(RatesError::MarketDataFetchFailed)
    }

    async fn fetch_market_data_internal(
        self: Arc<Self>,
        currency: &str,
    ) -> Result<MarketData, RatesError> {
        let start = Instant::now();

        let unit = match self.fiat_units.get(currency) {
            Some(unit) => unit.clone(),
            None => return Err(RatesError::UnsupportedCurrency),
        };

        let (price_str, used_source) = self
            .clone()
            .resolve_price_with_fallback(&unit)
            .await?;

        if let Ok(rate) = price_str.parse::<f64>() {
            let data = MarketData {
                price: format!("{} {:.0}", unit.symbol, rate),
                rate: rate,
                source: used_source.to_string(),
            };
            log::debug!("Market data fetched in {:?}", start.elapsed());
            return Ok(data);
        }

        Err(RatesError::PriceParseFailed(price_str))
    }
}

#[cfg_attr(feature = "bindings", uniffi::export)]
impl MarketAPI {
    #[cfg_attr(feature = "bindings", uniffi::constructor)]
    pub fn new() -> Result<Arc<Self>, RatesError> {
        let json_str = include_str!("../assets/fiatUnits.json");
        let fiat_units: HashMap<String, FiatUnit> = serde_json::from_str(json_str)
            .map_err(|e| RatesError::SerdeJsonError(e.to_string()))?;
        Ok(Arc::new(Self {
            fiat_units,
            client: Client::new(),
        }))
    }

    pub async fn fetch_market_data(
        self: Arc<Self>,
        currency: &str,
    ) -> Result<MarketData, RatesError> {
        #[cfg(feature = "bindings")]
        {
            let _self = self.clone();
            let currency = currency.to_string();
            async_utility::task::spawn(
                async move { _self.fetch_market_data_internal(&currency).await },
            )
            .join()
            .await
            .expect("No async task issues")
        }

        #[cfg(not(feature = "bindings"))]
        {
            self.fetch_market_data_internal(currency).await
        }
    }
}

#[tokio::test]
async fn test_fallback_primary_success() -> Result<(), RatesError> {
    let unit = FiatUnit {
        end_point_key: "USD".to_string(),
        source: Source::Kraken,
        symbol: "$".to_string(),
    };

    let attempts = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Source>::new()));
    let attempts_closure = attempts.clone();

    let (price, used_source) = MarketAPI::resolve_price_with_fallback_with_fetcher(&unit, move |source, _key| {
        let attempts = attempts_closure.clone();
        let source = source.clone();
        async move {
            attempts.lock().unwrap().push(source.clone());
            if source == Source::Kraken {
                Ok(Some("60000.0".to_string()))
            } else {
                Ok(None)
            }
        }
    })
    .await?;

    assert_eq!(price, "60000.0");
    assert_eq!(used_source, Source::Kraken);
    assert_eq!(attempts.lock().unwrap().as_slice(), &[Source::Kraken]);
    Ok(())
}

#[tokio::test]
async fn test_fallback_primary_fail_then_success() -> Result<(), RatesError> {
    let unit = FiatUnit {
        end_point_key: "USD".to_string(),
        source: Source::Kraken,
        symbol: "$".to_string(),
    };

    let attempts = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Source>::new()));
    let attempts_closure = attempts.clone();

    let (price, used_source) = MarketAPI::resolve_price_with_fallback_with_fetcher(&unit, move |source, _key| {
        let attempts = attempts_closure.clone();
        let source = source.clone();
        async move {
            attempts.lock().unwrap().push(source.clone());
            match source {
                Source::Kraken => Err(RatesError::HttpRequest("kraken down".to_string())),
                Source::CoinGecko => Ok(Some("61000.0".to_string())),
                _ => Ok(None),
            }
        }
    })
    .await?;

    assert_eq!(price, "61000.0");
    assert_eq!(used_source, Source::CoinGecko);
    assert_eq!(
        attempts.lock().unwrap().as_slice(),
        &[Source::Kraken, Source::CoinGecko]
    );
    Ok(())
}

#[tokio::test]
async fn test_fallback_all_fail() {
    let unit = FiatUnit {
        end_point_key: "USD".to_string(),
        source: Source::Kraken,
        symbol: "$".to_string(),
    };

    let attempts = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Source>::new()));
    let attempts_closure = attempts.clone();

    let result = MarketAPI::resolve_price_with_fallback_with_fetcher(&unit, move |source, _key| {
        let attempts = attempts_closure.clone();
        let source = source.clone();
        async move {
            attempts.lock().unwrap().push(source);
            Ok(None)
        }
    })
    .await;

    assert!(matches!(result, Err(RatesError::MarketDataFetchFailed)));
    assert_eq!(
        attempts.lock().unwrap().as_slice(),
        &[Source::Kraken, Source::CoinGecko, Source::CoinDesk]
    );
}

#[tokio::test]
async fn test_market_data_fetch_eur() -> Result<(), RatesError> {
    let api = MarketAPI::new()?;

    let market_data = api.fetch_market_data("EUR").await?;
    println!("Market data EUR: {:?}", market_data);

    assert!(
        !market_data.price.is_empty(),
        "Price string should not be empty"
    );
    assert!(market_data.rate > 0.0, "Rate must be greater than 0");
    assert!(market_data.price.starts_with('€'), "EUR price should start with €");

    let amount = 3000.0;
    let btc = market_data.calculate_btc(amount);
    println!("You have {:0.8} BTC", btc);

    let sats = market_data.calculate_sats(amount);
    println!("You have {} sats", sats);

    Ok(())
}

#[tokio::test]
async fn test_market_data_fetch_usd() -> Result<(), RatesError> {
    let api = MarketAPI::new()?;

    let market_data = api.fetch_market_data("USD").await?;
    println!("Market data USD: {:?}", market_data);

    assert!(
        !market_data.price.is_empty(),
        "Price string should not be empty"
    );
    assert!(market_data.rate > 0.0, "Rate must be greater than 0");
    assert!(market_data.price.starts_with('$'), "USD price should start with $");

    let amount = 5000.0;
    let btc = market_data.calculate_btc(amount);
    println!("You have {:0.8} BTC", btc);

    let msats = market_data.calculate_millisats(amount);
    println!("You have {} msats", msats);

    Ok(())
}
