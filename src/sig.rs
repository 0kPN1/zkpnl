extern crate signatory;
extern crate signatory_dalek;
use signatory::ed25519::{Seed, PublicKey, Signature};
use signatory::public_key::PublicKeyed;
use signatory::signature::{Signer, Verifier};
use signatory_dalek::{Ed25519Signer, Ed25519Verifier};
use crate::Result;
use crate::model::Verifiable;

pub fn sign(msg: &str) -> Result<String> {
    match get_seed() {
        None => {
            println!("{}", "No ed25519 seed found or seed format invalid. Skip signing.");
            Ok("".to_string())
        }
        Some(seed) => {
            let signer = Ed25519Signer::from(&seed);
            let sig = signer.try_sign(msg.as_bytes())?;
            let pk = signer.public_key()?;
            let sig_str = base64::encode(&sig.to_bytes().to_vec());
            let sig = get_sig(&sig_str)?;
            let verifier = Ed25519Verifier::from(&pk);
            verifier.verify(msg.as_bytes(), &sig)?;
            Ok(sig_str)
        }
    }
}

pub fn verify_sig<V: Verifiable>(varifiables: &Vec<V>) -> Result<()> {
    if let Some(pk) = get_pub_key() {
        verify_sigs_with_pk(&pk, &varifiables)
    } else {
        println!("{}", "Ed25519 seed not found. Skip verifying signatures");
        Ok(())
    }
}

pub fn verify_sigs_with_pk<V: Verifiable>(pk: &PublicKey, varifiables: &[V]) -> Result<()> {
    println!("{}", "verify message signatures");
    for v in varifiables.iter() {
        verify_sig_with_pk(pk, v)?;
    }
    Ok(())
}

pub fn verify_sig_with_pk<V: Verifiable>(pk: &PublicKey, varifiable: &V) -> Result<()> {
    let sig = get_sig(varifiable.sig())?;
    let verifier = Ed25519Verifier::from(pk);
    verifier.verify(varifiable.hash().as_bytes(), &sig)?;
    Ok(())
}

pub fn get_pub_key() -> Option<PublicKey> {
    get_seed().as_ref()
        .map(Ed25519Signer::from).as_ref()
        .map(Ed25519Signer::public_key)
        .map(|r| r.expect("get public key failed. please check seed format"))
}

pub fn get_pub_key_str() -> String {
    get_pub_key().as_ref()
        .map(base64::encode)
        .unwrap_or("".to_string())
}

pub fn get_pub_key_from_str(s: &str) -> Result<PublicKey> {
    let pk_vec = base64::decode(s)?;
    assert_eq!(pk_vec.len(), 32, "public key length incorrect");
    let mut pk_bytes: [u8; 32] = [0; 32];
    for (index, &byte) in pk_vec.iter().enumerate() {
        pk_bytes[index] = byte;
    }
    Ok(PublicKey::new(pk_bytes))
}

fn get_sig(s: &str) -> Result<Signature> {
    let sig_vec = base64::decode(s)?;
    assert_eq!(sig_vec.len(), 64, "signature length incorrect");
    let mut sig_bytes: [u8; 64] = [0; 64];
    for (index, &byte) in sig_vec.iter().enumerate() {
        sig_bytes[index] = byte;
    }
    Ok(Signature::new(sig_bytes))
}

fn get_seed() -> Option<Seed> {
    let seed_str: &'static str = crate::ZKPNL_CONFIG.ed25519_seed;
    if seed_str.is_empty() {
        None
    } else if let Ok(seed_bytes) = base64::decode(seed_str) {
        Seed::from_bytes(seed_bytes)
    } else {
        None
    }
}