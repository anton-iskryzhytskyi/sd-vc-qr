use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct SdJwtHeader {
    pub alg: String,
    pub typ: String,
    pub kid: String,
}

#[derive(Serialize, Deserialize)]
pub struct SdJwtPayload {
    pub iss: String,
    pub iat: i64,
    pub nbf: i64,
    pub _sd_alg: String,
    pub jti: String,
    pub vc: Value,
}

pub struct SdJwtComponents {
    pub header: Value,
    pub payload: Value,
    pub signature: Vec<u8>,
    pub disclosures: Vec<String>,
}
