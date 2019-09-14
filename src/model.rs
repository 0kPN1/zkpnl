use linked_hash_map::LinkedHashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, FixedOffset};
use bulletproofs::r1cs::LinearCombination;
use crate::{collection, core, digest, sig};
use crate::core::{Integerize, Deintegerize};
use crate::extension::{R1CSConfig, ProverExt};
use crate::{Result, ZKPNL_CONFIG};

pub type TradeMap = LinkedHashMap<String, Vec<Trade>>;
pub type TradeMsgMap = LinkedHashMap<String, Vec<TradeMsg>>;
pub type LCTradeMap = LinkedHashMap<String, Vec<(f64, LinearCombination)>>;
pub type I64TradeMap = LinkedHashMap<String, Vec<(f64, i64)>>;
pub type PriceMap = LinkedHashMap<String, f64>;
pub type PortMap = LinkedHashMap<String, i64>;
pub type LCPortMap = LinkedHashMap<String, LinearCombination>;
pub type PortCmtMap = LinkedHashMap<String, String>;
pub type PortBlndMap = LinkedHashMap<String, String>;

#[derive(Deserialize)]
pub struct ZKPNLConfig {
    pub transcript: &'static str,
    pub record_path: &'static str,
    pub price_path: &'static str,
    pub album_path: &'static str,
    pub proof_path: &'static str,
    pub bitmex: Vec<&'static str>,
    pub binance: Vec<&'static str>,
    pub ed25519_seed: &'static str,
    pub time_zone: i32,
    pub capital: f64,
}

impl ZKPNLConfig {
    pub fn is_option(&self, inst: &str) -> bool {
        !self.bitmex.contains(&inst) && !self.binance.contains(&inst)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MarketPrice {
    pub time: DateTime<FixedOffset>,
    pub market_price: PriceMap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TradeMsg {
    pub time: DateTime<FixedOffset>,
    pub r#type: TradeType,
    pub prev_hash: String,
    pub symbol: String,
    pub price: f64,
    pub qty: String,
    pub pnl: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trade {
    pub time: DateTime<FixedOffset>,
    pub r#type: TradeType,
    pub symbol: String,
    pub price: f64,
    pub qty: i64,
    pub qty_blnd: String,
    /// cumulative pnl since first trade
    pub pnl: f64,
    pub pnl_blnd: String,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum TradeType {
    Inherit, Trade, Deliver
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Record {
    pub hash: String,
    pub sig: String,
    pub msg: TradeMsg,
    pub trade: Trade,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlindedRecord {
    pub hash: String,
    pub sig: String,
    pub msg: TradeMsg,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnapshotMsg {
    pub time: DateTime<FixedOffset>,
    pub prev_hash: String,
    pub capital: f64,
    /// pnl since previous snapshot
    pub pnl: f64,
    pub log_return: f64,
    pub portfolio: PortCmtMap,
    /// records since previous snapshot
    pub records: Vec<BlindedRecord>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnapshotBlnd {
    pub time: DateTime<FixedOffset>,
    pub portfolio: PortMap,
    pub portfolio_blnd: PortBlndMap,
    pub records: Vec<Record>,
    pub market_price: PriceMap,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub hash: String,
    pub sig: String,
    pub msg: SnapshotMsg,
    pub snapshot_blnd: SnapshotBlnd,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BlindedSnapshot {
    pub hash: String,
    pub sig: String,
    pub msg: SnapshotMsg,
    pub market_price: PriceMap,
}

impl TradeType {
    pub fn new(s: &str) -> Option<TradeType> {
        match s {
            "inherit" => Some(TradeType::Inherit),
            "trade" | "" => Some(TradeType::Trade),
            "deliver" => Some(TradeType::Deliver),
            _ => None,
        }
    }
}

impl Record {
    pub fn new(time: DateTime<FixedOffset>, symbol: &str, qty: i64, price: f64, r#type: TradeType,
                   records: &[Record], price_map: &PriceMap) -> Result<Record> {
        let genesis_hash = digest::sha256(&ZKPNL_CONFIG.transcript);
        let prev_hash = records.last().map_or(&genesis_hash, |r|&r.hash).to_string();
        let mut trade_map = collection::get_i64_trade_map(&records);
        let qty = match r#type {
            TradeType::Deliver => -core::calc_size(&trade_map[symbol]),
            _ => qty,
        };
        if let Some(trade_vec) = trade_map.get_mut(symbol) {
            trade_vec.push((price, qty));
        } else {
            trade_map.insert(symbol.to_string(), vec![(price, qty)]);
        }
        let pnl = core::calc_total_pnl(&trade_map, &price_map).deintegerize();

        let mut config = R1CSConfig::default();
        let mut prover = config.make_prover();
        let (qty_cmt, qty_blnd) = prover.commit_quantity(qty);
        let (pnl_cmt, pnl_blnd) = prover.commit_quantity(i64::integerize(pnl));

        let msg = TradeMsg {
            time, r#type, price, prev_hash,
            symbol: symbol.to_string(),
            qty: qty_cmt,
            pnl: pnl_cmt,
        };
        let trade = Trade {
            time, r#type,
            symbol: symbol.to_string(),
            price, qty, qty_blnd, pnl, pnl_blnd
        };
        let hash = digest::sha256(String::from(&msg).as_ref());
        println!("{}", "sign hash");
        let sig = sig::sign(&hash)?;
        Ok(Record { hash, sig, msg, trade })
    }
}

impl Snapshot {
    pub fn new(time: DateTime<FixedOffset>, album: &[Snapshot], mut records: Vec<Record>, price_map: &PriceMap) -> Result<Snapshot> {
        let genesis_hash = digest::sha256(&ZKPNL_CONFIG.transcript);
        let prev_hash = album.last().map_or(&genesis_hash, |s|&s.hash).to_string();
        records.retain(|r|{
            album.last().map_or(true, |s|r.msg.time > s.msg.time)
        });
        let mut trade_map = collection::get_i64_trade_map(&records);
        if !album.is_empty() {
            let prev_port = &album.last().unwrap().snapshot_blnd.portfolio;
            let prev_price = &album.last().unwrap().snapshot_blnd.market_price;
            core::inherit_portfolio(&prev_port, &mut trade_map, &prev_price);
        }
        let curt_port = core::calc_portfolio(&trade_map);
        let pnl = core::calc_total_pnl(&trade_map, &price_map).deintegerize();

        let mut config = R1CSConfig::default();
        let mut prover = config.make_prover();
        let mut port_cmt: PortBlndMap = LinkedHashMap::new();
        let mut port_blnd: PortBlndMap = LinkedHashMap::new();
        for (symbol, size) in &curt_port {
            let (size_cmt, size_blnd) = prover.commit_quantity(*size);
            port_cmt.insert(symbol.clone(), size_cmt);
            port_blnd.insert(symbol.clone(), size_blnd);
        }
        let snapshot = SnapshotMsg {
            time, prev_hash, pnl,
            capital: ZKPNL_CONFIG.capital,
            log_return: f64::ln((pnl + ZKPNL_CONFIG.capital) / ZKPNL_CONFIG.capital),
            portfolio: port_cmt,
            records: records.iter().map(BlindedRecord::from).collect(),
        };
        let snapshot_blnd = SnapshotBlnd {
            time, records,
            portfolio: curt_port,
            portfolio_blnd: port_blnd,
            market_price: price_map.clone(),
        };
        let hash = digest::sha256(String::from(&snapshot).as_ref());
        println!("{}", "sign hash");
        let sig = sig::sign(&hash)?;
        Ok(Snapshot { hash, sig, msg: snapshot, snapshot_blnd })
    }
}

pub trait Verifiable {
    fn hash(&self) -> &str;
    fn sig(&self) -> &str;
    fn msg(&self) -> String;
    fn prev_hash(&self) -> &str;
}

impl Verifiable for Record {
    fn hash(&self) -> &str {
        &self.hash
    }
    fn sig(&self) -> &str {
        &self.sig
    }
    fn msg(&self) -> String {
        String::from(&self.msg)
    }
    fn prev_hash(&self) -> &str {
        &self.msg.prev_hash
    }
}

impl Verifiable for BlindedRecord {
    fn hash(&self) -> &str {
        &self.hash
    }
    fn sig(&self) -> &str {
        &self.sig
    }
    fn msg(&self) -> String {
        String::from(&self.msg)
    }
    fn prev_hash(&self) -> &str {
        &self.msg.prev_hash
    }
}

impl Verifiable for Snapshot {
    fn hash(&self) -> &str {
        &self.hash
    }
    fn sig(&self) -> &str {
        &self.sig
    }
    fn msg(&self) -> String {
        String::from(&self.msg)
    }
    fn prev_hash(&self) -> &str {
        &self.msg.prev_hash
    }
}

impl Verifiable for BlindedSnapshot {
    fn hash(&self) -> &str {
        &self.hash
    }
    fn sig(&self) -> &str {
        &self.sig
    }
    fn msg(&self) -> String {
        String::from(&self.msg)
    }
    fn prev_hash(&self) -> &str {
        &self.msg.prev_hash
    }
}

impl From<&Record> for BlindedRecord {
    fn from(unblinded: &Record) -> BlindedRecord {
        let r = unblinded.clone();
        BlindedRecord { hash: r.hash, sig: r.sig, msg: r.msg }
    }
}

impl From<&Snapshot> for BlindedSnapshot {
    fn from(unblinded: &Snapshot) -> BlindedSnapshot {
        let s = unblinded.clone();
        BlindedSnapshot { hash: s.hash, sig: s.sig, msg: s.msg, market_price: s.snapshot_blnd.market_price }
    }
}

impl From<&TradeMsg> for String {
    fn from(m: &TradeMsg) -> String {
        serde_json::to_string(m).unwrap()
    }
}

impl From<&Trade> for String {
    fn from(t: &Trade) -> String {
        serde_json::to_string(t).unwrap()
    }
}

impl From<&SnapshotMsg> for String {
    fn from(s: &SnapshotMsg) -> String {
        serde_json::to_string(s).unwrap()
    }
}

impl From<&SnapshotBlnd> for String {
    fn from(s: &SnapshotBlnd) -> String {
        serde_json::to_string(s).unwrap()
    }
}