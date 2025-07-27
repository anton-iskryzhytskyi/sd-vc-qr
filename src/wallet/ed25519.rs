use crate::sd_jwt::{issuer::SdJwtIssuer, verifier::SdJwtVerifier};
use crate::wallet::{DidKeys, IssuanceParams, SignatureAlgorithm, Wallet};
#[allow(unused_imports)]
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use multibase::{Base, encode};
use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
#[allow(unused_imports)]
use serde_json::{Value, json};
use std::error::Error as StdError;
use unsigned_varint::encode as varint_encode;
use rand_core::OsRng;

/// A Wallet implementation using the Ed25519 signature algorithm
pub struct Ed25519Wallet {
    // Configuration fields will go here
}

/// A signature algorithm implementation for Ed25519
pub struct Ed25519Algorithm;

impl SignatureAlgorithm for Ed25519Algorithm {
    fn algorithm_name(&self) -> &'static str {
        "EdDSA"
    }

    fn proof_type(&self) -> &'static str {
        "Ed25519Signature2020"
    }

    fn multicodec_prefix(&self) -> u64 {
        0xed // Ed25519 multicodec prefix
    }

    fn sign_data(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
        // Convert private key to SigningKey
        let signing_key = SigningKey::from_bytes(
            private_key.try_into()
                .map_err(|_| "Invalid Ed25519 private key length")?
        );

        // Sign the data
        let signature = signing_key.sign(data);

        Ok(signature.to_bytes().to_vec())
    }

    fn verify_signature(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Box<dyn StdError>> {
        // Convert public key to VerifyingKey
        let verifying_key = VerifyingKey::from_bytes(
            public_key.try_into()
                .map_err(|_| "Invalid Ed25519 public key length")?
        ).map_err(|e| format!("Invalid Ed25519 public key: {:?}", e))?;

        // Convert signature bytes to Signature
        let signature = Signature::from_bytes(
            signature.try_into()
                .map_err(|_| "Invalid Ed25519 signature length")?
        );

        // Verify the signature
        match verifying_key.verify(data, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Wallet for Ed25519Wallet {
    fn generate_did(&self) -> Result<(String, DidKeys), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = Ed25519Algorithm;

        // Generate a new Ed25519 keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Prepare multicodec prefix
        let mut prefix = varint_encode::u64_buffer();
        let prefix = varint_encode::u64(algorithm.multicodec_prefix(), &mut prefix);

        // Create payload and encode
        let mut payload = prefix.to_vec();
        payload.extend_from_slice(verifying_key.as_bytes());
        let mb = encode(Base::Base58Btc, &payload);

        // Form the DID
        let did = format!("did:key:{}", mb);

        // Create DidKeys with public and private keys
        let keys = DidKeys::new(
            verifying_key.as_bytes().to_vec(), 
            signing_key.as_bytes().to_vec()
        );

        Ok((did, keys))
    }

    fn issue_sd_vc_jwt(
        &self,
        params: IssuanceParams,
    ) -> Result<(String, Value), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = Ed25519Algorithm;

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
        let algorithm = Ed25519Algorithm;

        // Use the shared verifier component
        SdJwtVerifier::verify(&token, &algorithm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::{DidKeyResolver, FieldsAmount, FieldsSize, IssuanceConfig, Wallet};

    #[test]
    fn test_verify_sd_vc_jwt() {
        let wallet = Ed25519Wallet {};

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
        println!("\n=== Ed25519 JWT ===");
        println!("{}", jwt);
        println!("===================\n");
        
        // Verify the JWT
        let is_valid = wallet.verify_sd_vc_jwt(jwt).unwrap();
        assert!(is_valid, "JWT verification failed");
    }

    #[test]
    fn test_generate_did() {
        let generator = Ed25519Wallet {};
        let (did, keys) = generator.generate_did().unwrap();

        assert!(did.starts_with("did:key:"));
        assert_eq!(keys.pub_key.len(), 32); // Ed25519 public key is 32 bytes
        assert_eq!(keys.priv_key.len(), 32); // Ed25519 private key is 32 bytes
    }

    #[test]
    fn test_extract_pubkey_from_did() {
        let generator = Ed25519Wallet {};
        let algorithm = Ed25519Algorithm;
        let (did, keys) = generator.generate_did().unwrap();

        let extracted_pubkey =
            DidKeyResolver::extract_pubkey_from_did(&did, algorithm.multicodec_prefix()).unwrap();

        assert_eq!(extracted_pubkey, keys.pub_key, "Public key mismatch");
    }

    #[test]
    fn test_sign_and_verify_data() {
        let algorithm = Ed25519Algorithm;
        let wallet = Ed25519Wallet {};
        
        // Generate keys
        let (_did, keys) = wallet.generate_did().unwrap();
        
        // Test data to sign
        let test_data = b"Hello, Ed25519 world!";
        
        // Sign the data
        let signature = algorithm.sign_data(test_data, &keys.priv_key).unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");
        assert_eq!(signature.len(), 64, "Ed25519 signature should be 64 bytes");
        
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
        let algorithm = Ed25519Algorithm;
        
        assert_eq!(algorithm.algorithm_name(), "EdDSA");
        assert_eq!(algorithm.proof_type(), "Ed25519Signature2020");
        assert_eq!(algorithm.multicodec_prefix(), 0xed);
    }

    #[test]
    fn test_issue_sd_vc_jwt() {
        let wallet = Ed25519Wallet {};

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
        let (jwt, disclosures) = wallet.issue_sd_vc_jwt(params).unwrap();
        
        // Log the JWT and disclosures for manual verification
        println!("\n=== Ed25519 Full SD-JWT VC ===");
        println!("JWT: {}", jwt);
        println!("Disclosures: {}", serde_json::to_string_pretty(&disclosures).unwrap());
        println!("==============================\n");

        // Basic validation
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts separated by dots");

        // Decode and parse the header
        let header_json = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        let header: Value = serde_json::from_str(&header_json).unwrap();

        // Basic header validations
        assert_eq!(header["alg"], "EdDSA");
        assert_eq!(header["typ"], "vc+sd-jwt");

        // Decode and parse the payload
        let payload_json = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        let payload: Value = serde_json::from_str(&payload_json).unwrap();

        // Basic payload validations
        assert!(payload.get("iss").is_some());
        assert!(payload.get("credentialSubject").is_some());
        assert!(payload.get("validFrom").is_some());
        assert!(payload.get("validUntil").is_some());
        assert!(payload["_sd_alg"] == "sha-256");
        assert!(
            payload["type"]
                .as_array()
                .unwrap()
                .contains(&json!("TestCredential"))
        );

        // Verify disclosures
        assert!(disclosures.is_array());
        let disclosures_arr = disclosures.as_array().unwrap();
        assert!(!disclosures_arr.is_empty());
    }
}