use crate::Error;
use alloy_primitives::{Address, FixedBytes, B256, U256};
use k256::{
    ecdsa::{signature::hazmat::PrehashSigner, SigningKey},
    elliptic_curve::sec1::ToEncodedPoint,
    elliptic_curve::FieldBytes,
    EncodedPoint, PublicKey, Secp256k1,
};
use tiny_keccak::{Hasher, Sha3};

pub fn sha3<T: AsRef<[u8]>>(data: T) -> B256 {
    let mut hasher = Sha3::v256();
    hasher.update(data.as_ref());
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    B256::from_slice(&output)
}

pub fn u256_to_signing_key(pk: &U256) -> Result<SigningKey, Error> {
    SigningKey::from_slice(pk.as_le_slice()).map_err(|e| e.into())
}

pub fn addr(private_key: &SigningKey) -> Address {
    let verifying_key = private_key.verifying_key();
    let encoded_point: EncodedPoint = PublicKey::from(verifying_key).to_encoded_point(false);
    let pub_key_uncompressed = encoded_point.as_bytes();
    let mut hasher = Sha3::v256();
    let mut buf = [0u8; 32];
    hasher.update(&pub_key_uncompressed[1..]);
    hasher.finalize(&mut buf);
    Address::from_word(FixedBytes::from_slice(&buf))
}

/// Utility function mainly used for testing
pub fn sign_hash(hash: B256, private_key: &SigningKey) -> (u8, U256, U256) {
    let (recoverable_sig, recovery_id) = private_key.sign_prehash(hash.as_ref()).unwrap();

    let v = u8::from(recovery_id);

    let r_bytes: FieldBytes<Secp256k1> = recoverable_sig.r().into();
    let s_bytes: FieldBytes<Secp256k1> = recoverable_sig.s().into();
    let r = U256::from_be_slice(r_bytes.as_slice());
    let s = U256::from_be_slice(s_bytes.as_slice());

    (v, r, s)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_addr() {
        let pk = U256::from(100);
        let pk = u256_to_signing_key(&pk).unwrap();
        assert_eq!(
            Address::from_str("0x2a6669a8bec28af42dceff73070cdb2246adfb22").unwrap(),
            addr(&pk)
        );
    }
}
