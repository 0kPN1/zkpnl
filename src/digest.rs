use crypto::digest::Digest;
use crypto::sha2::Sha256;
use crate::model::Verifiable;

pub fn sha256(str: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.input_str(str);
    hasher.result_str()
}

pub fn verify_msg_hashes<V: Verifiable>(verifiables: &[V]) {
    println!("{}", "verify message hash");
    for v in verifiables {
        verify_msg_hash(v);
    }
}

pub fn verify_msg_hash<V: Verifiable>(verifiable: &V) {
    if verifiable.hash() != sha256(&verifiable.msg()) {
        panic!("verify message hash failed at {}", verifiable.hash())
    }
}

pub fn verify_hash_chain<V: Verifiable>(verifiables: &[V]) {
    let hashes: Vec<String> = verifiables.iter().skip(1)
        .map(|r|{
            r.prev_hash().replacen("\u{200b}", "", 1)
        }).collect();
    let message_jsons: Vec<String> = verifiables.iter()
        .map(V::msg).collect();
    for (p, h) in message_jsons.iter().zip(hashes) {
        if sha256(&p) != h {
            panic!("verify hash chain failed at {}", h)
        }
    };
}

pub fn verify_hash_chain_since_genesis<V: Verifiable>(genesis_text: &str, verifiables: &[V]) {
    println!("{}", "verify hash chain");
    let hashes: Vec<String> = verifiables.iter()
        .map(|r|{
            r.prev_hash().replacen("\u{200b}", "", 1)
        }).collect();
    let message_jsons: Vec<String> = verifiables.iter()
        .map(V::msg).collect();
    let plains = vec![vec![genesis_text.to_string()], message_jsons].concat();
    for (p, h) in plains.iter().zip(hashes) {
        if sha256(&p) != h {
            panic!("verify hash chain failed at {}", h)
        }
    };
}