use crate::sd_jwt::{issuer::SdJwtIssuer, verifier::SdJwtVerifier};
use crate::wallet::{DidKeys, IssuanceParams, SignatureAlgorithm, Wallet};
#[allow(unused_imports)]
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use k256::{
    EncodedPoint, SecretKey,
    ecdsa::{
        Signature, SigningKey,
        signature::{Signer, Verifier},
    },
};
use multibase::{Base, encode};
use rand_core::OsRng;
use serde_json::Value;
use std::error::Error as StdError;
use unsigned_varint::encode as varint_encode;

/// A Wallet implementation using the secp256k1 elliptic curve
pub struct Secp256k1Wallet;

/// A signature algorithm implementation for secp256k1
pub struct Secp256k1Algorithm;

impl SignatureAlgorithm for Secp256k1Algorithm {
    fn algorithm_name(&self) -> &'static str {
        "ES256K"
    }

    fn proof_type(&self) -> &'static str {
        "EcdsaSecp256k1Signature2019"
    }

    fn multicodec_prefix(&self) -> u64 {
        0xe7 // secp256k1 multicodec prefix
    }

    fn sign_data(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
        // Convert private key to a SigningKey
        let secret_key = SecretKey::from_slice(private_key)?;
        let signing_key = SigningKey::from(secret_key);

        // Sign the data
        let signature: Signature = signing_key.sign(data);

        // Convert to DER format for storage
        Ok(signature.to_der().as_bytes().to_vec())
    }

    fn verify_signature(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Box<dyn StdError>> {
        // Create a verifying key from the public key
        let verify_key = k256::ecdsa::VerifyingKey::from_sec1_bytes(public_key)
            .map_err(|e| format!("Invalid public key: {}", e))?;

        // Create a signature object from the signature data
        let signature = k256::ecdsa::Signature::from_der(signature)
            .map_err(|e| format!("Invalid signature format: {}", e))?;

        // Verify the signature
        Ok(verify_key.verify(data, &signature).is_ok())
    }
}

impl Wallet for Secp256k1Wallet {
    fn generate_did(&self) -> Result<(String, DidKeys), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = Secp256k1Algorithm;

        // 1. Generate a new secp256k1 signing key (private key)
        let signing_key = SigningKey::random(&mut OsRng);
        let sk_bytes = signing_key.to_bytes().to_vec();

        // 2. Derive the verifying (public) key and its compressed bytes
        let verify_key = signing_key.verifying_key();
        let pub_point: EncodedPoint = verify_key.to_encoded_point(true);
        let pk_bytes = pub_point.as_bytes().to_vec();

        // 3. Prepare multicodec prefix for secp256k1-pub
        let mut buf = varint_encode::u64_buffer();
        let prefix = varint_encode::u64(algorithm.multicodec_prefix(), &mut buf);

        // 4. Concatenate prefix and public key bytes, then multibase encode
        let mut payload = Vec::new();
        payload.extend_from_slice(prefix);
        payload.extend_from_slice(&pk_bytes);
        let mb = encode(Base::Base58Btc, &payload);

        // 5. Form the DID
        let did = format!("did:key:{}", mb);

        // 6. Construct DidKeys struct with raw key bytes
        let did_keys = DidKeys::new(pk_bytes, sk_bytes);

        Ok((did, did_keys))
    }

    fn issue_sd_vc_jwt(
        &self,
        params: IssuanceParams,
    ) -> Result<(String, Value), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = Secp256k1Algorithm;

        // Build the unsigned VC
        let unsigned = self.build_vc(&params)?;
        let subject = match &unsigned.credential_subject {
            Value::Object(map) => map,
            _ => return Err("credentialSubject must be an object".into()),
        };

        // Use the shared issuer component
        SdJwtIssuer::issue(
            serde_json::to_value(&unsigned)?,
            subject,
            &params.issuer_did,
            &params.private_key,
            &algorithm,
        )
    }

    fn verify_sd_vc_jwt(&self, token: String) -> Result<bool, Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = Secp256k1Algorithm;

        // Use the shared verifier component
        SdJwtVerifier::verify(&token, &algorithm)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::wallet::{FieldsAmount, FieldsSize, IssuanceConfig, Wallet};

    #[test]
    fn test_generate_did() {
        let wallet = Secp256k1Wallet;
        let (did, did_keys) = wallet.generate_did().unwrap();

        assert!(did.starts_with("did:key:"));
        assert_eq!(did_keys.pub_key.len(), 33); // Compressed public key length
        assert_eq!(did_keys.priv_key.len(), 32); // Private key length
    }

    #[test]
    fn test_sign_and_verify_data() {
        let algorithm = Secp256k1Algorithm;
        let wallet = Secp256k1Wallet {};
        
        // Generate keys
        let (_did, keys) = wallet.generate_did().unwrap();
        
        // Test data to sign
        let test_data = b"Hello, secp256k1 world!";
        
        // Sign the data
        let signature = algorithm.sign_data(test_data, &keys.priv_key).unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");
        
        // Verify the signature with correct public key
        let is_valid = algorithm.verify_signature(test_data, &signature, &keys.pub_key).unwrap();
        assert!(is_valid, "Signature verification should succeed with correct key");
        
        // Verify signature fails with wrong data
        let wrong_data = b"Wrong data";
        let is_invalid = algorithm.verify_signature(wrong_data, &signature, &keys.pub_key).unwrap();
        assert!(!is_invalid, "Signature verification should fail with wrong data");
        
        // Verify signature fails with wrong public key
        let (_wrong_did, wrong_keys) = wallet.generate_did().unwrap();
        let is_invalid_key = algorithm.verify_signature(test_data, &signature, &wrong_keys.pub_key).unwrap();
        assert!(!is_invalid_key, "Signature verification should fail with wrong public key");
    }

    #[test]
    fn test_algorithm_properties() {
        let algorithm = Secp256k1Algorithm;
        
        assert_eq!(algorithm.algorithm_name(), "ES256K");
        assert_eq!(algorithm.proof_type(), "EcdsaSecp256k1Signature2019");
        assert_eq!(algorithm.multicodec_prefix(), 0xe7);
    }

    #[test]
    fn test_issue_sd_vc_jwt() {
        let wallet = Secp256k1Wallet;

        // First generate a DID and keys
        let (issuer_did, keys) = wallet.generate_did().unwrap();

        // Create issuance parameters
        let params = IssuanceParams {
            issuer_did,
            method: "TestCredential".to_string(),
            private_key: keys.priv_key,
            config: IssuanceConfig {
                fields_amount: FieldsAmount::Small,
                fields_size: FieldsSize::Small,
                demo_vc: false,
                seed: 12345,
            },
        };

        // Issue the SD-JWT VC
        let (jwt, _disclosures) = wallet.issue_sd_vc_jwt(params).unwrap();
        
        // Log the JWT for manual verification
        println!("\n=== secp256k1 JWT ===");
        println!("{}", jwt);
        println!("=====================\n");

        // Basic validation
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts separated by dots");

        // Decode and parse the header
        let header_json = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        let header: Value = serde_json::from_str(&header_json).unwrap();

        // Basic header validations
        assert_eq!(header["alg"], "ES256K");
        assert_eq!(header["typ"], "vc+sd-jwt");

        // Decode and parse the payload
        let payload_json = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        let payload: Value = serde_json::from_str(&payload_json).unwrap();

        // Basic payload validations
        assert!(payload.get("iss").is_some());
        assert!(payload.get("credentialSubject").is_some());
        assert!(payload.get("validFrom").is_some());
        assert!(payload.get("validUntil").is_some());
        assert!(
            payload["type"]
                .as_array()
                .unwrap()
                .contains(&json!("TestCredential"))
        );
    }

    #[test]
    fn test_verify_sd_vc_jwt() {
        let wallet = Secp256k1Wallet;

        // First, we need to generate a DID and keys
        let (issuer_did, keys) = wallet.generate_did().unwrap();

        // Create issuance parameters
        let params = IssuanceParams {
            issuer_did: issuer_did.clone(),
            method: "TestCredential".to_string(),
            private_key: keys.priv_key.clone(),
            config: IssuanceConfig {
                fields_amount: FieldsAmount::Small,
                fields_size: FieldsSize::Small,
                demo_vc: false,
                seed: 12345,
            },
        };

        // Issue a SD-JWT VC
        let (jwt, _) = wallet.issue_sd_vc_jwt(params).unwrap();
        
        // Log the JWT for manual verification
        println!("\n=== secp256k1 Verification JWT ===");
        println!("{}", jwt);
        println!("==================================\n");

        let result = wallet.verify_sd_vc_jwt(jwt).unwrap();

        assert!(result, "JWT verification failed");
    }
}
