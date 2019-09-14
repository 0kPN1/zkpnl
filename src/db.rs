use std::collections::HashMap;
use std::io::ErrorKind;
use std::fs::{read_to_string, write};
use serde_json::{from_str, to_string_pretty, Value};
use csv::Writer;
use crate::digest::{verify_msg_hashes, verify_hash_chain_since_genesis};
use crate::model::{MarketPrice, Record, Snapshot};
use crate::proof::ZKPNLProof;
use crate::sig::verify_sig;
use crate::{ZKPNL_CONFIG, Result};
use crate::constants::PROTOCOL_VERSION;
use crate::report::SNPReport;

pub fn read_price() -> Result<Vec<MarketPrice>> {
    println!("{}", "read price file");
    let string = read_or_write_default("[]", ZKPNL_CONFIG.price_path);
    println!("{}", "parse price file");
    let price_map: Vec<MarketPrice> = from_str(&string)?;
    Ok(price_map)
}

pub fn read_record() -> Result<Vec<Record>> {
    println!("{}", "read record");
    let string = read_or_write_default("[]", ZKPNL_CONFIG.record_path);
    println!("{}", "parse record");
    let records: Vec<Record> = from_str(&string)?;
    verify_msg_hashes(&records);
    verify_hash_chain_since_genesis(&ZKPNL_CONFIG.transcript, &records);
    verify_sig(&records)?;
    Ok(records)
}

pub fn read_album() -> Result<Vec<Snapshot>> {
    println!("{}", "read album");
    let string = read_or_write_default("[]", ZKPNL_CONFIG.album_path);
    println!("{}", "parse album");
    let album: Vec<Snapshot> = from_str(&string)?;
    verify_msg_hashes(&album);
    verify_hash_chain_since_genesis(&ZKPNL_CONFIG.transcript, &album);
    verify_sig(&album)?;
    Ok(album)
}

pub fn read_proof(path: &str) -> Result<ZKPNLProof> {
    println!("{}", "read proof");
    let string: String = read_to_string(path)?;
    println!("{}", "parse proof");
    let map: HashMap<String, Value> = from_str(&string)?;
    if let Some(v) = map.get("protocol_version").and_then(|v|v.as_u64()) {
        if v == PROTOCOL_VERSION as u64 {
            let proof: ZKPNLProof = from_str(&string)?;
            Ok(proof)
        } else {
            panic!("proof file version incompatible")
        }
    } else {
        panic!("proof file format unrecognized")
    }
}

pub fn write_price(market_prices: Vec<MarketPrice>) -> Result<()> {
    println!("{}", "serialize price");
    let market_price_json = to_string_pretty(&market_prices)?;
    println!("{}", "write price");
    write(ZKPNL_CONFIG.price_path, market_price_json)?;
    Ok(())
}

pub fn write_record(records: Vec<Record>) -> Result<()> {
    println!("{}", "serialize record");
    let record_json = to_string_pretty(&records)?;
    println!("{}", "write record");
    write(ZKPNL_CONFIG.record_path, record_json)?;
    Ok(())
}

pub fn write_album(album: Vec<Snapshot>) -> Result<()> {
    println!("{}", "serialize album");
    let album_json = to_string_pretty(&album)?;
    println!("{}", "write album");
    write(ZKPNL_CONFIG.album_path, album_json)?;
    Ok(())
}

pub fn write_proof(proof: ZKPNLProof) -> Result<()> {
    println!("{}", "serialize proof");
    let proof_json = to_string_pretty(&proof)?;
    let start = proof.previous_snapshot.map(|s|s.msg.time)
        .or(proof.current_snapshot.msg.records.first().map(|r|r.msg.time))
        .map(|t|t.format("%F-%H%M%S").to_string())
        .unwrap_or("initial".to_string());
    let end = proof.current_snapshot.msg.time.format("%F-%H%M%S").to_string();
    let path = format!("{}proof_from_{}_to_{}.json", ZKPNL_CONFIG.proof_path, start, end);
    println!("write proof to path: {}", path);
    write(path, proof_json)?;
    Ok(())
}

pub fn write_snp_report(reports: Vec<SNPReport>) -> Result<()> {
    if reports.is_empty() {
        println!("{}", "no snapshot to export");
        return Ok(())
    }
    let start = reports.first().unwrap().time.format("%F-%H%M%S").to_string();
    let end = reports.last().unwrap().time.format("%F-%H%M%S").to_string();
    let path = format!("data/snapshot_from_{}_to_{}.csv", start, end);
    let mut wtr = Writer::from_path(path)?;
    println!("{}", "exporting snapshot");
    for r in reports {
        wtr.serialize(r)?;
    }
    wtr.flush()?;
    println!("{}", "completed");
    Ok(())
}

fn read_or_write_default(default: &str, path: &str) -> String {
    match read_to_string(path) {
        Ok(string) => string,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => match write(path, default) {
                Ok(_) => default.to_string(),
                Err(e) => panic!("tried to create file but there was a problem: {:?}", e),
            },
            other_error => panic!("there was a problem opening the file: {:?}", other_error),
        },
    }
}