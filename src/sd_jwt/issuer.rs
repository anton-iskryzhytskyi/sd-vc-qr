use crate::wallet::SignatureAlgorithm;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rand::RngCore;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::error::Error as StdError;

pub struct SdJwtIssuer;

impl SdJwtIssuer {
    pub fn issue<T: SignatureAlgorithm>(
        unsigned_vc: Value,
        subject: &Map<String, Value>,
        issuer_did: &str,
        private_key: &[u8],
        algorithm: &T,
    ) -> Result<(String, Value), Box<dyn StdError>> {
        // Create disclosures and digests
        let (disclosures, sd_digests) = Self::create_disclosures_and_digests(subject)?;

        // Create JWT Header
        let header = json!({
            "alg": algorithm.algorithm_name(),
            "typ": "vc+sd-jwt",
            "kid": format!("{}#keys-1", issuer_did)
        });

        let now = Utc::now();
        let valid_from = now.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
        let valid_until = (now + chrono::Duration::days(365 * 5)).format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();

        // Create JWT Payload - flattened VC structure with v2 fields
        let payload = json!({
            "_sd_alg": "sha-256",
            "@context": unsigned_vc["@context"],
            "credentialSubject": {
                "_sd": sd_digests
            },
            "exp": (now + chrono::Duration::days(365 * 5)).timestamp(),
            "iat": now.timestamp(),
            "id": unsigned_vc["id"],
            "iss": issuer_did,
            "issuer": {
                "id": issuer_did
            },
            "sub": unsigned_vc["id"],
            "type": unsigned_vc["type"],
            "validFrom": valid_from,
            "validUntil": valid_until
        });

        // Encode header and payload
        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header)?.as_bytes());
        let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&payload)?.as_bytes());
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        // Sign the input using the algorithm
        let signature = algorithm.sign_data(signing_input.as_bytes(), private_key)?;
        let sig_b64 = URL_SAFE_NO_PAD.encode(&signature);

        // Construct the JWT and return with disclosures
        let jwt = format!(
            "{}.{}.{}~{}",
            header_b64,
            payload_b64,
            sig_b64,
            disclosures.join("~")
        );

        Ok((jwt, json!(disclosures)))
    }

    /// Creates disclosures and digests for SD-JWT
    pub fn create_disclosures_and_digests(
        subject: &Map<String, Value>,
    ) -> Result<(Vec<String>, Vec<String>), Box<dyn StdError>> {
        let mut disclosures: Vec<String> = Vec::new();
        let mut sd_digests: Vec<String> = Vec::new();

        for (k, v) in subject.iter() {
            // Generate random salt
            let mut salt = [0u8; 16];
            rand::thread_rng().fill_bytes(&mut salt);
            let salt_b64 = URL_SAFE_NO_PAD.encode(salt);

            // Create disclosure: [salt, name, value]
            let disclosure = json!([salt_b64, k, v]);
            let disclosure_str = serde_json::to_string(&disclosure)?;
            let dis_b64 = URL_SAFE_NO_PAD.encode(disclosure_str.as_bytes());
            disclosures.push(dis_b64.clone());

            // Compute digest = sha256(disclosure)
            let mut hasher = Sha256::new();
            hasher.update(dis_b64.as_bytes());
            let digest = hasher.finalize();
            let digest_b64 = URL_SAFE_NO_PAD.encode(digest);
            sd_digests.push(digest_b64);
        }

        Ok((disclosures, sd_digests))
    }
}
