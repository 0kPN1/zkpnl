use linked_hash_map::LinkedHashMap;
use bulletproofs::r1cs::{LinearCombination, Prover, Verifier};
use bulletproofs::PedersenGens;
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::CompressedRistretto;
use merlin::Transcript;
use crate::model::{TradeMap, TradeMsgMap, LCTradeMap, PortMap, PortCmtMap, PortBlndMap, LCPortMap};

pub trait ScalarExt {
    fn from_i64(i: i64) -> Scalar;
}

impl ScalarExt for Scalar {
    fn from_i64(i: i64) -> Scalar {
        if i >= 0 {
            Scalar::from(i as u64)
        } else {
            -Scalar::from(-i as u64)
        }
    }
}

pub trait ProverExt {
    fn commit_quantity(&mut self, quantity: i64) -> (String, String);
    fn commit_trade_map(&mut self, trade_map: &TradeMap) -> LCTradeMap;
    fn commit_port_map(&mut self, port_map: &PortMap, port_blnd_map: &PortBlndMap) -> LCPortMap;
}

impl<'a> ProverExt for Prover<'a, 'a> {
    fn commit_quantity(&mut self, int: i64) -> (String, String) {
        let blinding = Scalar::random(&mut rand::thread_rng());
        let (commitment, _) = self.commit(Scalar::from_i64(int), blinding);
        let commitment = base64::encode(&commitment.to_bytes());
        let blinding = base64::encode(&blinding.to_bytes());
        (commitment, blinding)
    }

    fn commit_trade_map(&mut self, trade_map: &TradeMap) -> LCTradeMap {
        trade_map.iter().fold(LinkedHashMap::new(), |mut acc, (inst, trades)| {
            let pairs: Vec<(f64, LinearCombination)> = trades.iter().map(|t| {
                (t.price, self.commit(Scalar::from_i64(t.qty), get_scalar(&t.qty_blnd)).1.into())
            }).collect();
            acc.insert(inst.clone(), pairs);
            acc
        })
    }

    fn commit_port_map(&mut self, port_map: &PortMap, port_blnd_map: &PortBlndMap) -> LCPortMap {
        let mut lc_port_map: LCPortMap = LinkedHashMap::new();
        for (symbol, qty) in port_map {
            let variable = self.commit(Scalar::from_i64(*qty), get_scalar(&port_blnd_map[symbol])).1;
            lc_port_map.insert(symbol.clone(), variable.into());
        }
        lc_port_map
    }
}

pub trait VerifierExt {
    fn commit_trade_map(&mut self, message_map: TradeMsgMap) -> LCTradeMap;
    fn commit_port_map(&mut self, port_cmt_map: &PortCmtMap) -> LCPortMap;
}

impl<'a> VerifierExt for Verifier<'a> {
    fn commit_trade_map(&mut self, message_map: TradeMsgMap) -> LCTradeMap {
        message_map.iter().fold(LinkedHashMap::new(), |mut acc, (symbol, messages)| {
            let pairs: Vec<(f64, LinearCombination)> = messages.iter().map(|m| {
                let bytes = base64::decode(&m.qty).unwrap();
                let commitment = CompressedRistretto::from_slice(&bytes);
                (m.price, self.commit(commitment).into())
            }).collect();
            acc.insert(symbol.clone(), pairs);
            acc
        })
    }

    fn commit_port_map(&mut self, port_cmt_map: &PortCmtMap) -> LCPortMap {
        let mut lc_port_map: LCPortMap = LinkedHashMap::new();
        for (symbol, cmt) in port_cmt_map {
            let bytes = base64::decode(cmt).unwrap();
            let commitment = CompressedRistretto::from_slice(&bytes);
            let variable = self.commit(commitment);
            lc_port_map.insert(symbol.clone(), variable.into());
        }
        lc_port_map
    }
}

fn get_scalar(base64_str: &str) -> Scalar {
    let vec = base64::decode(base64_str).unwrap();
    assert_eq!(vec.len(), 32, "scalar length incorrect");
    let mut bytes: [u8; 32] = [0; 32];
    for (index, &byte) in vec.iter().enumerate() {
        bytes[index] = byte;
    }
    Scalar::from_bits(bytes)
}

pub struct R1CSConfig {
    pub pc_gens: PedersenGens,
    transcript: Transcript,
}

impl R1CSConfig {
    pub fn new(transcript: &'static str) -> R1CSConfig {
        R1CSConfig {
            pc_gens: PedersenGens::default(),
            transcript: Transcript::new(transcript.as_bytes()),
        }
    }

    pub fn make_prover(&mut self) -> Prover {
        Prover::new(&self.pc_gens, &mut self.transcript)
    }

    pub fn make_verifier(&mut self) -> Verifier {
        Verifier::new(&mut self.transcript)
    }
}

impl Default for R1CSConfig {
    fn default() -> R1CSConfig {
        R1CSConfig::new(crate::ZKPNL_CONFIG.transcript)
    }
}