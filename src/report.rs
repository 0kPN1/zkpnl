use std::cmp::Ordering::Equal;
use serde::Serialize;
use linked_hash_map::LinkedHashMap;
use chrono::{DateTime, FixedOffset};
use crate::{api, core};
use crate::model::*;
use crate::time::*;
use crate::time::TimeRange::*;
use crate::collection::get_symbols;
use crate::core::Deintegerize;
use crate::ZKPNL_CONFIG;

#[derive(Serialize)]
pub struct SNPReport {
    pub hash: String,
    pub time: DateTime<FixedOffset>,
    pub capital: f64,
    pub pnl: f64,
    pub log_return: f64,
}

#[derive(PartialEq, PartialOrd)]
pub struct PNLReport {
    pub is_option: bool,
    pub symbol: String,
    pub cash_balance: f64,
    pub market_value: f64,
    pub pnl: f64,
    pub market_price: f64,
    pub size: i64,
}

pub fn get_pnl_report(trade_map: &I64TradeMap, price_map: &PriceMap) -> Vec<PNLReport> {
    let mut report: Vec<PNLReport> = trade_map.iter().map(|(inst, trades)|{
        PNLReport::new(inst, trades, price_map)
    }).collect();
    report.sort_by(|a, b|b.partial_cmp(a).unwrap_or(Equal));
    report
}

impl SNPReport {
    pub fn new(snapshot: &Snapshot) -> SNPReport {
        SNPReport {
            hash: snapshot.hash.clone(),
            time: snapshot.msg.time,
            capital: snapshot.msg.capital,
            pnl: snapshot.msg.pnl,
            log_return: snapshot.msg.log_return,
        }
    }
}

impl PNLReport {
    fn new(symbol: &str, trades: &[(f64, i64)], price_map: &PriceMap) -> PNLReport {
        let is_option = ZKPNL_CONFIG.is_option(symbol);
        let market_price = *price_map.get(symbol).unwrap_or(&0.0);
        let cash_balance = core::calc_cash_balance(trades);
        let market_value = core::calc_market_value(trades, market_price);
        let underlying_price = if is_option && !trades.is_empty() {
            price_map.get("XBTUSD").map(|p|*p).unwrap_or_else(||{
                api::fetch_price("XBTUSD").unwrap_or(1.0)
            })
        } else { 1.0 };
        PNLReport {
            is_option,
            symbol: symbol.to_string(),
            market_price: market_price / underlying_price,
            size: core::calc_size(trades),
            cash_balance: cash_balance.deintegerize() / underlying_price,
            market_value: market_value.deintegerize() / underlying_price,
            pnl: (cash_balance + market_value).deintegerize(),
        }
    }
}

/// market_time is the time of price_map2
/// price_map1 1/2 the one most close to the specific start/end time
pub struct RangeFilteredPriceMap {
    pub market_time: DateTime<FixedOffset>,
    pub price_map1: PriceMap,
    pub price_map2: PriceMap,
}

impl RangeFilteredPriceMap {
    pub fn new(range: &TimeRange, market_prices: &[MarketPrice], rftm: &RangeFilteredTradeMap) -> RangeFilteredPriceMap {
        let market_time = match range {
            UpToNow | UpToNowSince(_) => now(),
            _ => rftm.last_trade_time,
        };
        let price_map1 = market_prices.iter().find(|mp|{
            mp.time == rftm.first_trade_time
        }).unwrap().market_price.clone();
        let price_map2 = match range {
            UpToNow | UpToNowSince(_)=> {
                let symbols = get_symbols(&market_prices);
                api::fetch_price_map(symbols).expect("fetch market price failed")
            },
            _ => {
                market_prices.iter().find(|mp|{
                    mp.time == rftm.last_trade_time
                }).unwrap().market_price.clone()
            }
        };
        RangeFilteredPriceMap { market_time, price_map1, price_map2 }
    }
}

/// trade_map 1/2 is from genesis to specific start/end time
pub struct RangeFilteredTradeMap {
    pub first_trade_time: DateTime<FixedOffset>,
    pub last_trade_time: DateTime<FixedOffset>,
    pub count: usize,
    pub i64_trade_map1: I64TradeMap,
    pub i64_trade_map2: I64TradeMap,
}

impl RangeFilteredTradeMap {
    pub fn new(range: &TimeRange, records: &[Record]) -> Option<RangeFilteredTradeMap> {
        let (start, end) = match range {
            Range(start, end) => (*start, *end),
            UpToLastSince(start) | UpToNowSince(start) => (*start, records.last().unwrap().msg.time),
            UpTo(end) => (records.first().unwrap().msg.time, *end),
            UpToLast | UpToNow => (records.first().unwrap().msg.time, records.last().unwrap().msg.time)
        };
        let mut slice1_len = 0usize;
        let mut time_vec: Vec<DateTime<FixedOffset>> = vec![];
        let mut plain_trade_map1: I64TradeMap = LinkedHashMap::new();
        let mut plain_trade_map2: I64TradeMap = LinkedHashMap::new();
        for r in records {
            if r.msg.time < start {
                slice1_len += 1;
                let plain_trade_vec = plain_trade_map1.entry(r.trade.symbol.clone()).or_insert(vec![]);
                plain_trade_vec.push((r.trade.price, r.trade.qty));
            }
            if r.msg.time <= end {
                time_vec.push(r.msg.time);
                let plain_trade_vec = plain_trade_map2.entry(r.trade.symbol.clone()).or_insert(vec![]);
                plain_trade_vec.push((r.trade.price, r.trade.qty));
            }
        }
        if plain_trade_map1.is_empty() {
            for k in plain_trade_map2.keys() {
                plain_trade_map1.insert(k.to_string(), vec![]);
            }
        }
        let time_vec = time_vec.split_off(slice1_len);
        if time_vec.is_empty() {
            None
        } else {
            Some(RangeFilteredTradeMap {
                first_trade_time: *time_vec.first().unwrap(),
                last_trade_time: *time_vec.last().unwrap(),
                count: time_vec.len(),
                i64_trade_map1: plain_trade_map1,
                i64_trade_map2: plain_trade_map2,
            })
        }
    }
}