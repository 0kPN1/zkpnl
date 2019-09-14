pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub const PROTOCOL_VERSION: u32 = 2;

pub const HELP_INFO: &str = r#"
Zero-knowledge P&L Prover
USAGE:
    commit <symbol> <quantity> (<price> [force] | market)
    inherit <symbol> <quantity>
    deliver <symbol>
    snapshot
    prove
    verify [<proof_file>]
    show market (all [save] | <symbol>)
    show report [from <start>] [to (<end> | now)]
    show snapshot
    export snapshot
    version
where <start> and <end> is in format yyyyMMddHHmm
"#;

pub const INTEGERIZE_FACTOR: u64 = 1_000_000_000;

pub const ZKPNL_CONFIG_PATH: &str = "config.toml";