# zkpnl
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
where \<start\> and \<end\> is in format yyyyMMddHHmm
