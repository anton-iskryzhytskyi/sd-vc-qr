use serde_json::{json, Value};
use crate::did::DidGenerator;
use pqcrypto_dilithium::dilithium2;
use multibase::{encode, Base};
use unsigned_varint::encode as varint_encode;
use pqcrypto_traits::sign::{PublicKey};

/// A DID Generator implementation using the Dilithium2 post-quantum algorithm
pub struct Dilithium2DidGenerator {
    // Configuration fields will go here
}

impl DidGenerator for Dilithium2DidGenerator {
    fn generate_did(&self) -> Result<(String, Value), Box<dyn std::error::Error>> {
        let (pk, _sk) = dilithium2::keypair();

        let mut prefix = varint_encode::u64_buffer();
        let prefix = varint_encode::u64(0xEF01, &mut prefix);

        let mut payload = prefix.to_vec();
        payload.extend_from_slice(pk.as_bytes());

        let mb = encode(Base::Base58Btc, &payload);

        let did = format!("did:key:{}", mb);

        Ok((did, json!({
            "privateKey": pk.as_bytes(),
        })))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::did::DidGenerator;

    #[test]
    fn test_generate_did() {
        let generator = Dilithium2DidGenerator {};
        let (did, private_key) = generator.generate_did().unwrap();

        assert!(did.starts_with("did:key:"));
        assert!(private_key.is_object());
    }
}