use linked_hash_map::LinkedHashMap;
use crate::model::{Record, BlindedRecord, TradeMap, TradeMsgMap, I64TradeMap, MarketPrice};

pub fn get_trade_map(records: &[Record]) -> TradeMap {
    records.iter().fold(LinkedHashMap::new(), |mut acc, r| {
        let value = acc.entry(r.trade.symbol.clone()).or_insert(vec![]);
        value.push(r.trade.clone());
        acc
    })
}

pub fn get_trade_msg_map(records: &[BlindedRecord]) -> TradeMsgMap {
    records.iter().fold(LinkedHashMap::new(), |mut acc, r| {
        let value = acc.entry(r.msg.symbol.clone()).or_insert(vec![]);
        value.push(r.msg.clone());
        acc
    })
}

pub fn get_i64_trade_map(records: &[Record]) -> I64TradeMap {
    records.iter().fold(LinkedHashMap::new(), |mut acc, r| {
        let value = acc.entry(r.trade.symbol.clone()).or_insert(vec![]);
        value.push((r.trade.price, r.trade.qty));
        acc
    })
}

pub fn get_symbols(market_prices: &[MarketPrice]) -> Vec<&str> {
    market_prices.last().map_or(vec![], |mp|{
        mp.market_price.iter().map(|(s, _)|s.as_str()).collect()
    })
}