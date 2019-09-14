use std::collections::HashMap;
use linked_hash_map::LinkedHashMap;
use serde_json::{from_str, Value};
use reqwest::get;
use crate::model::PriceMap;
use crate::{Result, ZKPNL_CONFIG};

pub fn fetch_price_map(symbols: Vec<&str>) -> Result<PriceMap> {
    let mut price_map: PriceMap = LinkedHashMap::new();
    for s in symbols {
        println!("fetch market price of {}", s);
        price_map.insert(s.to_string(), fetch_price(s)?);
    }
    Ok(price_map)
}

pub fn fetch_price(symbol: &str) -> Result<f64> {
    if ZKPNL_CONFIG.bitmex.contains(&symbol) {
        bitmex(symbol)
    } else if ZKPNL_CONFIG.binance.contains(&symbol) {
        binance(symbol)
    } else {
        deribit(symbol)
    }
}

fn bitmex(symbol: &str) -> Result<f64> {
    let url = format!("https://www.bitmex.com/api/v1/instrument?symbol={}", symbol);
    let res_str = get(&url)?.text()?;
    let res_map: Vec<HashMap<String, Value>> = from_str(&res_str)?;
    let price = res_map[0]["lastPrice"].as_f64().unwrap();
    Ok(price)
}

fn binance(symbol: &str) -> Result<f64> {
    let url = format!("https://www.binance.com/api/v3/ticker/price?symbol={}", symbol);
    let res_str = get(&url)?.text()?;
    let res_map: HashMap<String, String> = from_str(&res_str)?;
    let price = res_map["price"].parse::<f64>()?;
    Ok(price)
}

fn deribit(symbol: &str) -> Result<f64> {
    let url = format!("https://www.deribit.com/api/v2/public/ticker?instrument_name={}", symbol);
    let res_str = get(&url)?.text()?;
    let res_map: HashMap<String, Value> = from_str(&res_str)?;
    let result_str = res_map.get("result").expect("instrument not found").to_string();
    let result_map: HashMap<String, Value> = from_str(&result_str)?;
    let mark_price = result_map["mark_price"].as_f64().unwrap();
    let underlying_price = result_map.get("delivery_price")
        .unwrap_or(&result_map["underlying_price"]).as_f64().unwrap();
    Ok(mark_price * underlying_price)
}
