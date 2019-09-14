use serde::{Deserialize, Serialize};
use bulletproofs::{BulletproofGens, PedersenGens};
use bulletproofs::r1cs::{LinearCombination, R1CSProof};
use curve25519_dalek::scalar::Scalar;
use crate::*;
use crate::model::*;
use crate::core::Integerize;
use crate::extension::{R1CSConfig, ProverExt, VerifierExt};

#[derive(Serialize, Deserialize)]
pub struct ZKPNLProof {
    pub protocol_version: u32,
    pub transcript: String,
    pub ed25519_pub_key: String,
    pub r1cs_proof: String,
    pub current_snapshot: BlindedSnapshot,
    pub previous_snapshot: Option<BlindedSnapshot>,
}

impl ZKPNLProof {
    pub fn new(previous: Option<&Snapshot>, current: &Snapshot) -> ZKPNLProof {
        let mut r1cs_config = R1CSConfig::default();
        let mut prover = r1cs_config.make_prover();
        let bp_gens = BulletproofGens::new(128, 1);

        let trade_map = collection::get_trade_map(&current.snapshot_blnd.records);
        let mut lc_trade_map = prover.commit_trade_map(&trade_map);
        if let Some(previous) = previous {
            let prev_lc_port_map = prover.commit_port_map(&previous.snapshot_blnd.portfolio, &previous.snapshot_blnd.portfolio_blnd);
            core::inherit_portfolio(&prev_lc_port_map, &mut lc_trade_map, &previous.snapshot_blnd.market_price);
        }
        let lc_pnl = core::calc_total_pnl::<LinearCombination, Scalar>(&lc_trade_map, &current.snapshot_blnd.market_price);
        let expected = Scalar::integerize(current.msg.pnl);
        constrain::equal(&mut prover, lc_pnl, expected);

        let curt_lc_port_map = prover.commit_port_map(&current.snapshot_blnd.portfolio, &current.snapshot_blnd.portfolio_blnd);
        let expected_lc_port_map = core::calc_portfolio::<LinearCombination, Scalar>(&lc_trade_map);
        for (symbol, lc_size) in curt_lc_port_map {
            constrain::equal(&mut prover, lc_size, expected_lc_port_map[&symbol].clone());
        }

        ZKPNLProof {
            protocol_version: constants::PROTOCOL_VERSION,
            transcript: ZKPNL_CONFIG.transcript.to_string(),
            ed25519_pub_key: sig::get_pub_key_str(),
            r1cs_proof: base64::encode(&prover.prove(&bp_gens).unwrap().to_bytes()),
            current_snapshot: BlindedSnapshot::from(current),
            previous_snapshot: previous.map(BlindedSnapshot::from),
        }
    }

    pub fn verify_r1cs(&self) -> Result<()> {
        // static transcript is required by R1CS so make one here using Box
        let transcript: &'static str = Box::leak(self.transcript.clone().into_boxed_str());
        let mut r1cs_config = R1CSConfig::new(transcript);
        let mut verifier = r1cs_config.make_verifier();

        let trade_map = collection::get_trade_msg_map(&self.current_snapshot.msg.records);
        let mut lc_trade_map = verifier.commit_trade_map(trade_map);
        if let Some(previous) = &self.previous_snapshot {
            let lc_port_map = verifier.commit_port_map(&previous.msg.portfolio);
            core::inherit_portfolio(&lc_port_map, &mut lc_trade_map, &previous.market_price);
        }
        let lc_pnl = core::calc_total_pnl::<LinearCombination, Scalar>(&lc_trade_map, &self.current_snapshot.market_price);
        let expected = Scalar::integerize(self.current_snapshot.msg.pnl);
        constrain::equal(&mut verifier, lc_pnl, expected);

        let curt_lc_port_map = verifier.commit_port_map(&self.current_snapshot.msg.portfolio);
        let expected_lc_port_map = core::calc_portfolio::<LinearCombination, Scalar>(&lc_trade_map);
        for (symbol, lc_size) in curt_lc_port_map {
            constrain::equal(&mut verifier, lc_size, expected_lc_port_map[&symbol].clone());
        }

        println!("{}", "verify r1cs proof");
        let proof_bytes = base64::decode(&self.r1cs_proof)?;
        let proof = R1CSProof::from_bytes(&proof_bytes).unwrap();
        let pc_gens = PedersenGens::default();
        let bp_gens = BulletproofGens::new(128, 1);
        let result = verifier.verify(&proof, &pc_gens, &bp_gens);
        match result {
            Ok(()) => println!("{}", "verify OK"),
            Err(e) => panic!("{}", e),
        };
        Ok(())
    }

    pub fn verify_hash(&self) -> Result<()> {
        digest::verify_msg_hashes(&self.current_snapshot.msg.records);
        println!("{}", "verify snapshot hash");
        digest::verify_msg_hash(&self.current_snapshot);
        if self.previous_snapshot.is_none() {
            digest::verify_hash_chain_since_genesis(&self.transcript, &self.current_snapshot.msg.records);
            if self.current_snapshot.prev_hash() != digest::sha256(&self.transcript) {
                panic!("verify initial snapshot hash chain failed")
            }
        } else {
            digest::verify_hash_chain(&self.current_snapshot.msg.records);
            if self.current_snapshot.prev_hash() != digest::sha256(&String::from(&self.previous_snapshot.as_ref().unwrap().msg)) {
                panic!("verify snapshot hash chain failed")
            }
        }
        Ok(())
    }

    pub fn verify_sig(&self) -> Result<()> {
        let pk = sig::get_pub_key_from_str(&self.ed25519_pub_key)?;
        sig::verify_sigs_with_pk(&pk, &self.current_snapshot.msg.records)?;
        println!("{}", "verify snapshot signature");
        sig::verify_sig_with_pk(&pk, &self.current_snapshot)?;
        Ok(())
    }
}