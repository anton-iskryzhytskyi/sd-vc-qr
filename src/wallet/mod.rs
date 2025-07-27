use chrono::Utc;
use rand::{RngCore, SeedableRng};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::error::Error as StdError;
use uuid::Uuid;

/// A struct containing the public and private keys for a DID
pub struct DidKeys {
    pub pub_key: Vec<u8>,
    pub priv_key: Vec<u8>,
}

impl DidKeys {
    pub fn new(pub_key: Vec<u8>, priv_key: Vec<u8>) -> Self {
        Self { pub_key, priv_key }
    }
}

/// Configuration for issuance functionality
pub struct IssuanceConfig {
    pub fields_amount: FieldsAmount,
    pub fields_size: FieldsSize,
    pub demo_vc: bool,
    pub seed: u32,
}

impl Default for IssuanceConfig {
    fn default() -> Self {
        Self {
            fields_amount: FieldsAmount::Small,
            fields_size: FieldsSize::Small,
            demo_vc: false,
            seed: 0,
        }
    }
}

/// Enum representing the amount of fields to include in a VC
#[derive(Clone, Copy, Debug)]
pub enum FieldsAmount {
    Small,
    Medium,
    Large,
}

/// Enum representing the size of fields to include in a VC
#[derive(Clone, Copy, Debug)]
pub enum FieldsSize {
    Small,
    Medium,
    Large,
}

impl std::fmt::Display for FieldsAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_fields", self.to_usize())
    }
}

impl std::fmt::Display for FieldsSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_bytes", self.to_usize())
    }
}

impl FieldsAmount {
    pub fn to_usize(&self) -> usize {
        match self {
            FieldsAmount::Small => 5,
            FieldsAmount::Medium => 50,
            FieldsAmount::Large => 500,
        }
    }
}

impl FieldsSize {
    pub fn to_usize(&self) -> usize {
        match self {
            FieldsSize::Small => 6,
            FieldsSize::Medium => 64,
            FieldsSize::Large => 640,
        }
    }
}

/// Parameters for issuing a Verifiable Credential
pub struct IssuanceParams {
    pub issuer_did: String,
    pub method: String,
    pub private_key: Vec<u8>,
    pub config: IssuanceConfig,
}
#[derive(Serialize)]
pub struct UnsignedVC {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    #[serde(rename = "type")]
    pub types: Vec<String>,
    pub issuer: String,
    #[serde(rename = "issuanceDate")]
    pub issuance_date: String,
    #[serde(rename = "expirationDate", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,
    #[serde(rename = "credentialSubject")]
    pub credential_subject: Value,
    #[serde(rename = "credentialSchema", skip_serializing_if = "Option::is_none")]
    pub credential_schema: Option<Value>,
}

// Common utilities for DID operations
pub struct DidKeyResolver;

impl DidKeyResolver {
    /// Extracts a public key from a DID
    pub fn extract_pubkey_from_did(
        did: &str,
        expected_code: u64,
    ) -> Result<Vec<u8>, Box<dyn StdError>> {
        // 1. Strip the did:key prefix
        let prefix = "did:key:";
        let mb_str = did
            .strip_prefix(prefix)
            .ok_or("DID does not start with 'did:key:'")?;

        // 2. Multibase decode (Base58-BTC)
        let (_base, data) = multibase::decode(mb_str)?;

        // 3. Varint decode multicodec prefix
        let (code, remainder) = unsigned_varint::decode::u64(&data)
            .map_err(|e| format!("Failed to decode varint: {}", e))?;
        if code != expected_code {
            return Err(format!(
                "Unexpected multicodec prefix: expected 0x{:x}, got 0x{:x}",
                expected_code, code
            )
            .into());
        }

        // 4. The remainder slice contains the raw public key bytes
        let pubkey = remainder.to_vec();
        Ok(pubkey)
    }
}

// Signature algorithm trait
pub trait SignatureAlgorithm {
    /// Algorithm name for JWT header
    fn algorithm_name(&self) -> &'static str;

    /// Proof type for VC proof
    fn proof_type(&self) -> &'static str;

    /// Expected multicodec prefix for DIDs
    fn multicodec_prefix(&self) -> u64;

    /// Signs data with the given private key
    fn sign_data(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, Box<dyn StdError>>;

    /// Verifies signature with the given public key
    fn verify_signature(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Box<dyn StdError>>;
}

pub mod dilithium2;
pub mod secp256k1;
pub mod p256;
pub mod ed25519;
pub mod falcon512;
pub mod sphincsplus128s;

/// A trait for wallet implementations.
///
/// Implementations of this trait provide functionality to generate
/// Decentralized Identifiers (DIDs) and associated keys.
pub trait Wallet {
    /// Generate a new DID and its associated keys.
    ///
    /// # Returns
    ///
    /// * `Result<(String, DidKeys), Box<dyn std::error::Error>>` - A tuple containing:
    ///   * A String representing the DID
    ///   * A DidKeys struct containing the public and private keys
    fn generate_did(&self) -> Result<(String, DidKeys), Box<dyn std::error::Error>>;

    /// Issue a Verifiable Credential in SD-JWT format.
    ///
    /// # Parameters
    ///
    /// * `params` - IssuanceParams containing issuer information and configuration
    ///
    /// # Returns
    ///
    /// * `Result<String, Box<dyn std::error::Error>>` - A string containing the SD-JWT VC
    fn issue_sd_vc_jwt(
        &self,
        params: IssuanceParams,
    ) -> Result<(String, Value), Box<dyn std::error::Error>>;

    fn verify_sd_vc_jwt(&self, token: String) -> Result<bool, Box<dyn std::error::Error>>;

    fn build_vc(&self, params: &IssuanceParams) -> Result<UnsignedVC, Box<dyn std::error::Error>> {
        // Context
        let context = vec!["https://www.w3.org/2018/credentials/v1".to_string()];
        // ID as UUID URN
        let id = format!("urn:uuid:{}", Uuid::new_v4());
        // Types: VerifiableCredential + custom method
        let mut types = vec!["VerifiableCredential".to_string()];
        types.push(params.method.clone());
        // Issuance date
        let issuance_date = Utc::now().to_rfc3339();
        // No expiration in this example
        let expiration_date = None;
        // Build credentialSubject
        let mut subject_map = Map::new();
        // Always include holder DID
        subject_map.insert("id".to_string(), json!(params.issuer_did));
        // Demo VC: single field
        if params.config.demo_vc {
            subject_map.insert("demo".to_string(), json!(true));
        } else {
            // Generate additional fields
            let amount = params.config.fields_amount.to_usize();
            let size = params.config.fields_size.to_usize();
            let mut rng = rand::rngs::StdRng::seed_from_u64(params.config.seed as u64);
            for i in 0..amount {
                let key = format!("field{}", i + 1);
                let val: String = (0..size)
                    .map(|_| (0x61u8 + (rng.next_u32() % 26) as u8) as char)
                    .collect();
                subject_map.insert(key, json!(val));
            }
        }
        let credential_subject = Value::Object(subject_map);
        let unsigned = UnsignedVC {
            context,
            id,
            types,
            issuer: params.issuer_did.clone(),
            issuance_date,
            expiration_date,
            credential_subject,
            credential_schema: None,
        };
        Ok(unsigned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::Wallet;

    #[test]
    fn test_build_vc() {
        let wallet = secp256k1::Secp256k1Wallet {};
        let params = IssuanceParams {
            issuer_did: "did:key:example".to_string(),
            method: "ExampleMethod".to_string(),
            private_key: vec![],
            config: IssuanceConfig::default(),
        };
        let vc = wallet.build_vc(&params).unwrap();

        assert_eq!(vc.issuer, "did:key:example");
        assert_eq!(vc.types[0], "VerifiableCredential");
    }
}
