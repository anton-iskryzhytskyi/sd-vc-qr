use crate::sd_jwt::{issuer::SdJwtIssuer, verifier::SdJwtVerifier};
use crate::wallet::{DidKeys, IssuanceParams, SignatureAlgorithm, Wallet};
#[allow(unused_imports)]
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use multibase::{Base, encode};
use pqcrypto_sphincsplus::sphincsshake128ssimple;
use pqcrypto_traits::sign::{DetachedSignature, PublicKey, SecretKey};
#[allow(unused_imports)]
use serde_json::{Value, json};
use std::error::Error as StdError;
use unsigned_varint::encode as varint_encode;

/// A Wallet implementation using the SPHINCS+-128s post-quantum algorithm
pub struct SphincsPLus128sWallet {
    // Configuration fields will go here
}

/// A signature algorithm implementation for SPHINCS+-128s
pub struct SphincsPLus128sAlgorithm;

impl SignatureAlgorithm for SphincsPLus128sAlgorithm {
    fn algorithm_name(&self) -> &'static str {
        "sphincs+128s"
    }

    fn proof_type(&self) -> &'static str {
        "SphincsPlusQuantumResistantSignature2023"
    }

    fn multicodec_prefix(&self) -> u64 {
        0xEF03 // SPHINCS+-128s multicodec prefix
    }

    fn sign_data(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>> {
        // Convert private key to a SecretKey
        let secret_key = sphincsshake128ssimple::SecretKey::from_bytes(private_key)
            .map_err(|e| format!("Failed to create SecretKey: {:?}", e))?;

        // Sign the data
        let signature = sphincsshake128ssimple::detached_sign(data, &secret_key);

        // Return the signature bytes
        Ok(signature.as_bytes().to_vec())
    }

    fn verify_signature(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Box<dyn StdError>> {
        // Convert public key to a PublicKey
        let pubkey = sphincsshake128ssimple::PublicKey::from_bytes(public_key)
            .map_err(|e| format!("Invalid SPHINCS+-128s public key: {:?}", e))?;

        // Convert signature to a DetachedSignature
        let signature = sphincsshake128ssimple::DetachedSignature::from_bytes(signature)
            .map_err(|e| format!("Invalid SPHINCS+-128s signature: {:?}", e))?;

        // Verify the signature
        let result = sphincsshake128ssimple::verify_detached_signature(&signature, data, &pubkey);

        Ok(result.is_ok())
    }
}

impl Wallet for SphincsPLus128sWallet {
    fn generate_did(&self) -> Result<(String, DidKeys), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = SphincsPLus128sAlgorithm;

        // Generate a new SPHINCS+-128s keypair
        let (pk, sk) = sphincsshake128ssimple::keypair();

        // Prepare multicodec prefix
        let mut prefix = varint_encode::u64_buffer();
        let prefix = varint_encode::u64(algorithm.multicodec_prefix(), &mut prefix);

        // Create payload and encode
        let mut payload = prefix.to_vec();
        payload.extend_from_slice(pk.as_bytes());
        let mb = encode(Base::Base58Btc, &payload);

        // Form the DID
        let did = format!("did:key:{}", mb);

        // Create DidKeys with public and private keys
        let keys = DidKeys::new(pk.as_bytes().to_vec(), sk.as_bytes().to_vec());

        Ok((did, keys))
    }

    fn issue_sd_vc_jwt(
        &self,
        params: IssuanceParams,
    ) -> Result<(String, Value), Box<dyn std::error::Error>> {
        // Get algorithm instance
        let algorithm = SphincsPLus128sAlgorithm;

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
        let algorithm = SphincsPLus128sAlgorithm;

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
        let wallet = SphincsPLus128sWallet {};

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
        println!("\n=== SPHINCS+-128s JWT ===");
        println!("{}", jwt);
        println!("=========================\n");
        
        // Verify the JWT
        let is_valid = wallet.verify_sd_vc_jwt(jwt).unwrap();
        assert!(is_valid, "JWT verification failed");
    }

    #[test]
    fn test_generate_did() {
        let generator = SphincsPLus128sWallet {};
        let (did, keys) = generator.generate_did().unwrap();

        assert!(did.starts_with("did:key:"));
        assert!(!keys.pub_key.is_empty());
        assert!(!keys.priv_key.is_empty());
    }

    #[test]
    fn test_extract_pubkey_from_did() {
        let generator = SphincsPLus128sWallet {};
        let algorithm = SphincsPLus128sAlgorithm;
        let (did, keys) = generator.generate_did().unwrap();

        let extracted_pubkey =
            DidKeyResolver::extract_pubkey_from_did(&did, algorithm.multicodec_prefix()).unwrap();

        assert_eq!(extracted_pubkey, keys.pub_key, "Public key mismatch");
    }

    #[test]
    fn test_sign_and_verify_data() {
        let algorithm = SphincsPLus128sAlgorithm;
        let wallet = SphincsPLus128sWallet {};
        
        // Generate keys
        let (_did, keys) = wallet.generate_did().unwrap();
        
        // Test data to sign
        let test_data = b"Hello, SPHINCS+-128s world!";
        
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
        let algorithm = SphincsPLus128sAlgorithm;
        
        assert_eq!(algorithm.algorithm_name(), "sphincs+128s");
        assert_eq!(algorithm.proof_type(), "SphincsPlusQuantumResistantSignature2023");
        assert_eq!(algorithm.multicodec_prefix(), 0xEF03);
    }

    #[test]
    fn test_issue_sd_vc_jwt() {
        let wallet = SphincsPLus128sWallet {};

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
        println!("\n=== SPHINCS+-128s Full SD-JWT VC ===");
        println!("JWT: {}", jwt);
        println!("Disclosures: {}", serde_json::to_string_pretty(&disclosures).unwrap());
        println!("====================================\n");

        // Basic validation
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have 3 parts separated by dots");

        // Decode and parse the header
        let header_json = String::from_utf8(URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        let header: Value = serde_json::from_str(&header_json).unwrap();

        // Basic header validations
        assert_eq!(header["alg"], "sphincs+128s");
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