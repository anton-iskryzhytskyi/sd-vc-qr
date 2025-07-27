use crate::sd_jwt::{issuer::SdJwtIssuer, verifier::SdJwtVerifier};
use crate::wallet::{DidKeys, IssuanceParams, SignatureAlgorithm, Wallet};
#[allow(unused_imports)]
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use p256::{
    EncodedPoint, SecretKey,
    ecdsa::{
        Signature, SigningKey,
        signature::{Signer, Verifier},
    },
    elliptic_curve::sec1::ToEncodedPoint,
};
use multibase::{Base, encode};
use rand_core::OsRng;
use serde_json::Value;
use std::error::Error as StdError;
use unsigned_varint::encode as varint_encode;

/// A Wallet implementation using the P-256 elliptic curve
pub struct P256Wallet;

/// A signature algorithm implementation for P-256
pub struct P256Algorithm;

impl SignatureAlgorithm for P256Algorithm {
    fn algorithm_name(&self) -> &'static str {
        "ES256"
    }

    fn proof_type(&self) -> &'static str {
        "EcdsaSecp256r1Signature2019"
    }

    fn multicodec_prefix(&self) -> u64 {
        0x1200 // P-256 multicodec prefix
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
        // Parse the signature from DER format
        let sig = Signature::from_der(signature)?;

        // Create verifying key from public key bytes
        let encoded_point = EncodedPoint::from_bytes(public_key)?;
        let verifying_key = p256::ecdsa::VerifyingKey::from_encoded_point(&encoded_point)?;

        // Verify the signature
        match verifying_key.verify(data, &sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl Wallet for P256Wallet {

    fn generate_did(&self) -> Result<(String, DidKeys), Box<dyn StdError>> {
        // Generate a new P-256 key pair
        let secret_key = SecretKey::random(&mut OsRng);
        let public_key = secret_key.public_key();

        // Get the encoded point (uncompressed format)
        let encoded_point = public_key.to_encoded_point(false);
        let public_key_bytes = encoded_point.as_bytes();

        // Create multicodec-encoded public key
        let mut multicodec_key = Vec::new();
        multicodec_key.extend_from_slice(&varint_encode::u64(0x1200, &mut [0; 10])[..2]); // P-256 multicodec
        multicodec_key.extend_from_slice(public_key_bytes);

        // Encode as multibase
        let multibase_key = encode(Base::Base58Btc, &multicodec_key);

        // Create DID
        let did = format!("did:key:{}", multibase_key);

        // Create DidKeys
        let keys = DidKeys::new(public_key_bytes.to_vec(), secret_key.to_bytes().to_vec());

        Ok((did, keys))
    }

    fn issue_sd_vc_jwt(
        &self,
        params: IssuanceParams,
    ) -> Result<(String, Value), Box<dyn StdError>> {
        // Get algorithm instance
        let algorithm = P256Algorithm;

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

    fn verify_sd_vc_jwt(&self, token: String) -> Result<bool, Box<dyn StdError>> {
        // Get algorithm instance
        let algorithm = P256Algorithm;

        // Use the shared verifier component
        SdJwtVerifier::verify(&token, &algorithm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_did() {
        let wallet = P256Wallet;
        let (did, keys) = wallet.generate_did().unwrap();

        // Basic DID format checks
        assert!(did.starts_with("did:key:"));
        assert!(!keys.priv_key.is_empty());
        assert!(!keys.pub_key.is_empty());
        
        // Check key lengths
        assert_eq!(keys.priv_key.len(), 32); // P-256 private key is 32 bytes
        assert_eq!(keys.pub_key.len(), 65); // Uncompressed P-256 public key is 65 bytes
    }

    #[test]
    fn test_sign_and_verify_data() {
        let algorithm = P256Algorithm;
        let wallet = P256Wallet {};
        
        // Generate keys
        let (_did, keys) = wallet.generate_did().unwrap();
        
        // Test data to sign
        let test_data = b"Hello, P-256 world!";
        
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
        let algorithm = P256Algorithm;
        
        assert_eq!(algorithm.algorithm_name(), "ES256");
        assert_eq!(algorithm.proof_type(), "EcdsaSecp256r1Signature2019");
        assert_eq!(algorithm.multicodec_prefix(), 0x1200);
    }

    #[test]
    fn test_issue_sd_vc_jwt() {
        let wallet = P256Wallet;

        // First, we need to generate a DID and keys
        let (issuer_did, keys) = wallet.generate_did().unwrap();

        let params = IssuanceParams {
            issuer_did: issuer_did.clone(),
            method: "TestCredential".to_string(),
            private_key: keys.priv_key,
            config: crate::wallet::IssuanceConfig::default(),
        };

        // Issue the JWT
        let (jwt, _disclosures) = wallet.issue_sd_vc_jwt(params).unwrap();
        
        // Log the JWT for manual verification
        println!("\n=== P-256 JWT ===");
        println!("{}", jwt);
        println!("=================\n");

        // Basic JWT format checks
        assert!(jwt.contains('.'));
        assert!(jwt.contains('~'));

        // Decode and parse the payload
        let parts: Vec<&str> = jwt.split('~').collect();
        let jwt_parts: Vec<&str> = parts[0].split('.').collect();
        assert_eq!(jwt_parts.len(), 3);

        let payload_json = String::from_utf8(URL_SAFE_NO_PAD.decode(jwt_parts[1]).unwrap()).unwrap();
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
        let wallet = P256Wallet;

        // First, we need to generate a DID and keys
        let (issuer_did, keys) = wallet.generate_did().unwrap();

        let params = IssuanceParams {
            issuer_did: issuer_did.clone(),
            method: "TestCredential".to_string(),
            private_key: keys.priv_key,
            config: crate::wallet::IssuanceConfig::default(),
        };

        // Issue the JWT
        let (jwt, _disclosures) = wallet.issue_sd_vc_jwt(params).unwrap();
        
        // Log the JWT for manual verification
        println!("\n=== P-256 Verification JWT ===");
        println!("{}", jwt);
        println!("==============================\n");

        // Verify the JWT
        let is_valid = wallet.verify_sd_vc_jwt(jwt).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_extract_pubkey_from_did() {
        use crate::wallet::DidKeyResolver;

        let wallet = P256Wallet;
        let (did, keys) = wallet.generate_did().unwrap();

        // Extract public key from DID
        let extracted_pubkey = DidKeyResolver::extract_pubkey_from_did(&did, 0x1200).unwrap();

        // Should match the original public key
        assert_eq!(extracted_pubkey, keys.pub_key);
    }
}