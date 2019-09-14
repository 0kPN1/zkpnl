#[macro_use]
extern crate lazy_static;

mod api;
mod cmd;
mod core;
mod collection;
mod constrain;
mod constants;
mod db;
mod digest;
mod extension;
mod model;
mod proof;
mod report;
mod sig;
mod time;

lazy_static! {
    static ref ZKPNL_CONFIG_STR: String = std::fs::read_to_string(constants::ZKPNL_CONFIG_PATH)
        .expect("please add a config file");
    pub static ref ZKPNL_CONFIG: model::ZKPNLConfig = toml::from_str(&ZKPNL_CONFIG_STR)
        .expect("please check config file format");
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    use crate::model::TradeType;
    let args_owned: Vec<String> = std::env::args().collect();
    let args: Vec<&str> = args_owned.iter().map(String::as_str).collect();
    match args.get(1) {
        Some(&"commit") => {
            let r#type = TradeType::Trade;
            if args.get(2).is_none() || args.get(3).is_none() || args.get(4).is_none() {
                println!("{}", constants::HELP_INFO);
            } else if args[4] == "market" {
                cmd::commit(r#type, &args[2], args[3].parse::<i64>()?, -1.0)?;
            } else {
                let price = args[4].parse::<f64>()?;
                if price >= 1.0 {
                    cmd::commit(r#type, &args[2], args[3].parse::<i64>()?, price)?;
                } else if price >= 0.0 {
                    if let Some(&"force") = args.get(4) {
                        cmd::commit(r#type, &args[2], args[3].parse::<i64>()?, price)?;
                    } else {
                        println!("{}", "error: price below 1 should use force flag: commit <symbol> <quantity> <price> force")
                    }
                } else {
                    println!("{}", "error: invalid price")
                }
            }
        },
        Some(&"inherit") => {
            if args.get(2).is_none() || args.get(3).is_none() {
                println!("{}", "please specify symbol and quantity following format:\ninherit <symbol> <quantity>");
            } else {
                cmd::commit(TradeType::Inherit, &args[2], args[3].parse::<i64>()?, -1.0)?;
            }
        },
        Some(&"deliver") => {
            if args.get(2).is_none() {
                println!("{}", "please specify symbol following format:\ndeliver <symbol>");
            } else {
                cmd::commit(TradeType::Deliver, &args[2], 0, -1.0)?;
            }
        },
        Some(&"snapshot") => {
            cmd::snapshot()?;
        },
        Some(&"prove") => {
            cmd::prove()?;
        },
        Some(&"verify") => {
            if let Some(proof_file_path) = args.get(2) {
                cmd::verify(proof_file_path)?;
            } else {
                cmd::verify_all()?;
            }
        },
        Some(&"show") => {
            match args.get(2) {
                Some(&"market") => {
                    match args.get(3) {
                        Some(&"all") => {
                            let saves = args.get(4) == Some(&"save");
                            cmd::show_market_all(saves)?;
                        },
                        Some(symbol) => cmd::show_market(symbol)?,
                        None => println!("{}", constants::HELP_INFO),
                    }
                },
                Some(&"snapshot") => {
                    cmd::show_snapshot()?;
                },
                Some(&"report") => {
                    let range = time::TimeRange::new(args.get(3), args.get(4), args.get(5), args.get(6))?;
                    cmd::show_report(range)?;
                },
                _ => println!("{}", constants::HELP_INFO),
            }
        },
        Some(&"export") => {
            match args.get(2) {
                Some(&"snapshot") => {
                    cmd::export_snapshot()?;
                },
                _ => println!("{}", constants::HELP_INFO),
            }
        }
        Some(&"version") => println!("version {}\nprotocol version {}", constants::VERSION, constants::PROTOCOL_VERSION),
        _ => println!("{}", constants::HELP_INFO),
    }
    Ok(())
}