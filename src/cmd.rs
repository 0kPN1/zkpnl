use crate::*;
use crate::model::*;
use crate::proof::ZKPNLProof;
use crate::time::TimeRange;

pub fn commit(r#type: TradeType, symbol: &str, qty: i64, price: f64) -> Result<()> {
    let time = time::now();
    let mut records: Vec<Record> = db::read_record()?;
    let mut market_prices = db::read_price()?;

    let mut symbols = collection::get_symbols(&market_prices);
    if !symbols.contains(&symbol) { symbols.push(symbol) };
    let price_map = api::fetch_price_map(symbols)?;
    let price = if price == -1.0 { price_map[symbol] } else { price };

    let new_record =  Record::new(time, symbol, qty, price, r#type, &records, &price_map)?;
    let new_hash = new_record.hash.clone();
    let sig = new_record.sig.clone();
    records.push(new_record);
    db::write_record(records)?;

    let new_market_price = MarketPrice { time, market_price: price_map };
    market_prices.push(new_market_price);
    db::write_price(market_prices)?;

    show_report(TimeRange::UpToLast)?;
    println!("hash: {}\nsig: {}", new_hash, sig);
    Ok(())
}

pub fn snapshot() -> Result<()> {
    let time = time::now();
    let records: Vec<Record> = db::read_record()?;
    if records.is_empty() {
        println!("{}", "no record. please commit first.");
        return Ok(())
    }
    let mut market_prices = db::read_price()?;
    let mut album = db::read_album()?;
    let symbols = collection::get_symbols(&market_prices);
    let price_map = api::fetch_price_map(symbols)?;
    let snapshot = model::Snapshot::new(time, &album, records, &price_map)?;

    let start = album.last()
        .map_or_else(||snapshot.snapshot_blnd.records.first().unwrap().msg.time, |a|a.msg.time);
    let pnl = snapshot.msg.pnl;
    let log_return = snapshot.msg.log_return;
    let port = snapshot.snapshot_blnd.portfolio.clone();
    let hash = snapshot.hash.clone();
    let sig = snapshot.sig.clone();

    album.push(snapshot);
    db::write_album(album)?;
    let new_market_price = MarketPrice { time, market_price: price_map };
    market_prices.push(new_market_price);
    db::write_price(market_prices)?;

    println!("\n{:^25}|{:^8}", "Instrument", "Size");
    for (symbol, size) in port {
        println!("{:^25}|{:^8}", symbol, size);
    }
    println!("\nFrom\t\t{}\nTo\t\t{}\nP&L\t\t{}\nLog Return\t{}", start, time, pnl, log_return);
    println!("\nhash: {}\nsig: {}", hash, sig);
    Ok(())
}

pub fn prove() -> Result<()> {
    let album = db::read_album()?;
    if album.is_empty() {
        println!("{}", "no snapshot. please take snapshot first.");
        return Ok(())
    }
    println!("{}", "generating initial snapshot proof");
    let proof = ZKPNLProof::new(None, album.first().unwrap());
    db::write_proof(proof)?;
    println!("{}", "generating snapshot proof");
    let album_tail = album.split_first().unwrap().1;
    for (previous, current) in album.iter().zip(album_tail) {
        let proof = ZKPNLProof::new(Some(previous), current);
        db::write_proof(proof)?;
    }
    println!("Write all {} snapshot proofs completed", album.len());
    Ok(())
}

pub fn verify(path: &str) -> Result<()> {
    let proof = db::read_proof(path)?;
    proof.verify_hash()?;
    proof.verify_sig()?;
    proof.verify_r1cs()?;
    Ok(())
}

pub fn verify_all() -> Result<()> {
    let mut count = 0;
    for entry in std::fs::read_dir(ZKPNL_CONFIG.proof_path)? {
        if let Some(path) = entry?.path().to_str() {
            count += 1;
            println!("verify {}", path);
            verify(path)?;
        }
    }
    if count == 0 {
        println!("no proof file found in path {}", ZKPNL_CONFIG.proof_path);
    } else {
        println!("Verify all {} proofs OK.", count);
    }
    Ok(())
}

pub fn show_market(symbol: &str) -> Result<()> {
    let price = api::fetch_price(symbol)?;
    println!("{}: {:>10.4} USD", symbol, price);
    Ok(())
}

pub fn show_market_all(saves: bool) -> Result<()> {
    let time = time::now();
    let mut market_prices = db::read_price()?;
    let mut symbols = collection::get_symbols(&market_prices);
    if symbols.is_empty() { symbols = vec!["XBTUSD"]; }
    let price_map = api::fetch_price_map(symbols)?;
    if saves {
        let new_market_price = MarketPrice { time, market_price: price_map.clone() };
        market_prices.push(new_market_price);
        db::write_price(market_prices)?;
    }
    for (symbol, price) in &price_map {
        if ZKPNL_CONFIG.is_option(symbol) {
            println!("{:<20} {:>10.4} BTC", symbol, price / price_map["XBTUSD"]);
        } else {
            println!("{:<20} {:>10.4} USD", symbol, price);
        }
    }
    Ok(())
}

pub fn show_snapshot() -> Result<()> {
    let album = db::read_album()?;
    println!("{}", "");
    fn table_row<S: std::fmt::Display>(c1: S, c2: S, c3: S, c4: S) -> String {
        format!("{:^10}|{:^35}|{:^16}|{:^16}", c1, c2, c3, c4)
    }
    fn usd(f: f64) -> String { format!("{:>10.1}", f) }
    fn log(f: f64) -> String { format!("{:>10.8}", f) }
    println!("{}", table_row("Hash", "Time", "P&L (USD)", "Log Return"));
    println!("{}", "----------------------------------------------------------------------");
    for snp in album {
        println!("{}", table_row(&snp.hash[..7], &snp.msg.time.to_rfc2822(), &usd(snp.msg.pnl), &log(snp.msg.log_return)));
    }
    println!("{}", "");
    Ok(())
}

pub fn export_snapshot() -> Result<()> {
    let album = db::read_album()?;
    let reports: Vec<report::SNPReport> = album.iter().map(report::SNPReport::new).collect();
    db::write_snp_report(reports)?;
    Ok(())
}

pub fn show_report(range: TimeRange) -> Result<()> {
    let records: Vec<Record> = db::read_record()?;
    let rftm = report::RangeFilteredTradeMap::new(&range, &records);
    if rftm.is_none() {
        println!("{}", "no record found in this range");
        return Ok(())
    }
    let rftm = rftm.unwrap();
    let market_prices = db::read_price()?;
    let rfpm = report::RangeFilteredPriceMap::new(&range, &market_prices, &rftm);
    let reports1 = report::get_pnl_report(&rftm.i64_trade_map1, &rfpm.price_map1);
    let reports2 = report::get_pnl_report(&rftm.i64_trade_map2, &rfpm.price_map2);
    let (mut usd_balance, mut btc_balance, mut usd_value, mut btc_value, mut total_pnl) = (0.0, 0.0, 0.0, 0.0, 0.0);
    fn usd(f: f64) -> String { format!("{:>10.1} USD", f) }
    fn btc(f: f64) -> String { format!("{:>10.4} BTC", f) }
    fn size(i: i64) -> String { format!("{:>5}", i) }
    fn table_row<S: std::fmt::Display>(c1: S, c2: S, c3: S, c4: S, c5: S, c6: S, c7: S) -> String {
        format!("{:^25}|{:^8}|{:^16}|{:^16}|{:^16}|{:^16}|{:^16}", c1, c2, c3, c4, c5, c6, c7)
    }
    println!("{}", "");
    println!("First trade\t{}", rftm.first_trade_time);
    println!("Last trade\t{}", rftm.last_trade_time);
    println!("Market price\t{}", rfpm.market_time);
    println!("{}", "");
    println!("{}", table_row("Instrument", "Size", "Market Price", "Avg. Price", "Cash Balance", "Market Value", "P&L"));
    println!("{}", "------------------------------------------------------------------------------------------------------------------------");
    for (r1, r2) in reports1.iter().zip(reports2) {
        let cb = r2.cash_balance - r1.cash_balance;
        let mv = r2.market_value - r1.market_value;
        let pnl = r2.pnl - r1.pnl;
        let avg_price = -cb / r2.size as f64;
        if r2.is_option {
            if r2.size != 0 { btc_balance += cb; }
            btc_value += mv;
            println!("{}", table_row(r2.symbol, size(r2.size), btc(r2.market_price), btc(avg_price), btc(cb), btc(mv), usd(pnl)));
        } else {
            if r2.size != 0 { usd_balance += cb; }
            usd_value += mv;
            println!("{}", table_row(r2.symbol, size(r2.size), usd(r2.market_price), usd(avg_price), usd(cb), usd(mv), usd(pnl)));
        }
        total_pnl += pnl;
    }
    println!("{}", "");
    println!("Number of trades: {}", rftm.count);
    println!("Total Cash Balance: {:.1} USD + {:.4} BTC (zero size instruments are not included)", usd_balance, btc_balance);
    println!("Total Market Value: {:.1} USD + {:.4} BTC", usd_value, btc_value);
    println!("Total P&L: {:.1} USD ", total_pnl);
    println!("{}", "");
    Ok(())
}