use crate::wallet::DidKeyResolver;
use crate::wallet::SignatureAlgorithm;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error as StdError;

pub struct SdJwtVerifier;

impl SdJwtVerifier {
    pub fn verify<T: SignatureAlgorithm>(
        token: &str,
        algorithm: &T,
    ) -> Result<bool, Box<dyn StdError>> {
        // Parse the JWT
        let (jwt_parts, parts) = Self::parse_jwt(token)?;

        // Verify JWT structure
        let (_header, payload, issuer_did) =
            Self::verify_jwt_structure(&jwt_parts, algorithm.algorithm_name())?;

        // Extract the signature
        let signature_data = URL_SAFE_NO_PAD
            .decode(jwt_parts[2])
            .map_err(|e| format!("Failed to decode signature: {}", e))?;

        // Get the public key from the DID
        let public_key_bytes =
            DidKeyResolver::extract_pubkey_from_did(&issuer_did, algorithm.multicodec_prefix())?;

        // Create the message that was signed
        let message = format!("{}.{}", jwt_parts[0], jwt_parts[1]);

        // Verify the signature using the algorithm
        let signature_valid =
            algorithm.verify_signature(message.as_bytes(), &signature_data, &public_key_bytes)?;

        // Verify disclosures if present
        if parts.len() > 1 {
            let disclosures_valid = Self::verify_disclosures(&parts, &payload)?;
            if !disclosures_valid {
                return Ok(false);
            }
        }

        // Return the verification result
        Ok(signature_valid)
    }

    /// Parses a JWT token into its components
    pub fn parse_jwt(token: &str) -> Result<(Vec<&str>, Vec<&str>), Box<dyn StdError>> {
        let parts: Vec<&str> = token.split('~').collect();
        let jwt_parts: Vec<&str> = parts[0].split('.').collect();

        if jwt_parts.len() != 3 {
            return Err("Invalid JWT format".into());
        }

        Ok((jwt_parts, parts))
    }

    /// Verifies the JWT structure (header, payload)
    pub fn verify_jwt_structure(
        jwt_parts: &[&str],
        expected_alg: &str,
    ) -> Result<(Value, Value, String), Box<dyn StdError>> {
        // Extract and decode the header
        let header_data = URL_SAFE_NO_PAD
            .decode(jwt_parts[0])
            .map_err(|e| format!("Failed to decode header: {}", e))?;
        let header: Value = serde_json::from_slice(&header_data)
            .map_err(|e| format!("Failed to parse header: {}", e))?;

        // Check algorithm
        if header["alg"] != expected_alg {
            return Err(format!("Unsupported algorithm: {}", header["alg"]).into());
        }

        // Extract the KID (key identifier) to get the public key
        let kid = header["kid"].as_str().ok_or("Missing kid in header")?;

        // Extract the DID from the KID
        let issuer_did = kid
            .split('#')
            .next()
            .ok_or("Invalid kid format")?
            .to_string();

        // Extract and decode the payload
        let payload_data = URL_SAFE_NO_PAD
            .decode(jwt_parts[1])
            .map_err(|e| format!("Failed to decode payload: {}", e))?;
        let payload: Value = serde_json::from_slice(&payload_data)
            .map_err(|e| format!("Failed to parse payload: {}", e))?;

        // Verify the issuer matches what's in the KID
        if payload["iss"] != issuer_did {
            return Err("Issuer mismatch".into());
        }

        Ok((header, payload, issuer_did))
    }

    /// Verifies the JWT disclosures
    pub fn verify_disclosures(parts: &[&str], payload: &Value) -> Result<bool, Box<dyn StdError>> {
        if parts.len() <= 1 {
            return Ok(true); // No disclosures to verify
        }

        // Get the selective disclosure algorithm
        let sd_alg = payload["_sd_alg"]
            .as_str()
            .ok_or("Missing _sd_alg in payload")?;

        if sd_alg != "sha-256" {
            return Err(format!("Unsupported SD algorithm: {}", sd_alg).into());
        }

        // Get SD digests from the payload (v2 structure)
        let sd_array = match &payload["credentialSubject"]["_sd"] {
            Value::Array(array) => array,
            _ => return Err("Invalid _sd format in payload".into()),
        };

        // Verify each disclosure
        for i in 1..parts.len() {
            let disclosure_b64 = parts[i];

            // Compute the digest of the disclosure
            let mut hasher = Sha256::new();
            hasher.update(disclosure_b64.as_bytes());
            let actual_digest = hasher.finalize();
            let actual_digest_b64 = URL_SAFE_NO_PAD.encode(actual_digest);

            // Check if this digest is in the _sd array
            if !sd_array.iter().any(|d| d.as_str() == Some(&actual_digest_b64)) {
                return Err(format!("Digest {} not found in _sd array", actual_digest_b64).into());
            }
        }

        Ok(true)
    }
}
