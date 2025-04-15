use std::sync::Arc;

use reqwest::Client;
use url::Url;

use alloy_primitives::hex;

use crate::gio_error::GIOError;

#[repr(u32)]
pub enum GIODomain {
    GetStorage = 0x27,
    GetAccount = 0x29,
    GetImage = 0x2a,
    PreimageHint = 0x2e,
}

impl GIODomain {
    pub fn to_bytes(self) -> [u8; 4] {
        (self as u32).to_ne_bytes()
    }
}

#[repr(u16)]
pub enum GIOHint {
    EthCodePreimage = 1,
    EthBlockPreimage = 2,
}

impl GIOHint {
    pub fn to_bytes(self) -> [u8; 2] {
        (self as u16).to_ne_bytes()
    }
}

#[repr(u8)]
pub enum GIOHash {
    Keccak256 = 2,
}

impl GIOHash {
    pub fn to_bytes(self) -> [u8; 1] {
        (self as u8).to_ne_bytes()
    }
}

pub struct GIOResponse {
    pub code: u32,
    pub data: Vec<u8>,
}

impl GIOResponse {
    pub fn is_ok(&self) -> bool {
        self.code == 200
    }
}

pub struct GIOClient {
    url: Url,
    client: Arc<Client>,
}

impl GIOClient {
    pub fn new(url: Url) -> Self {
        let client = Arc::new(Client::new());
        Self { url, client }
    }

    pub async fn emit_gio(
        &self,
        domain: GIODomain,
        input: &Vec<u8>,
    ) -> Result<GIOResponse, GIOError> {
        let hex_data = hex::encode_prefixed(input);
        let request = GIOServerRequest {
            domain: domain as u32,
            id: hex_data,
        };

        let mut body = Vec::<u8>::new();
        serde_json::to_writer(&mut body, &request);

        let resp = self
            .client
            .post(self.url.clone())
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;

        let resp_body = resp
            .bytes()
            .await
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?
            .to_vec();

        let resp_json: GIOServerResponse = serde_json::from_slice(&resp_body)
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;

        let resp_data =
            hex::decode(resp_json.response).map_err(|err| GIOError::EmitFailed(err.to_string()))?;

        Ok(GIOResponse {
            code: resp_json.response_code,
            data: resp_data,
        })
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GIOServerRequest {
    domain: u32,
    id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GIOServerResponse {
    pub response_code: u32,
    pub response: String,
}
