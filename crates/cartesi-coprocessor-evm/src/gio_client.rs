use hyper::{client::HttpConnector, Body, Client, Request};
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
        (self as u32).to_le_bytes()
    }
}

#[repr(u8)]
pub enum GIOHint {
    EthCodePreimage = 1,
    EthBlockPreimage = 2,
}

impl GIOHint {
    pub fn to_bytes(self) -> [u8; 1] {
        (self as u8).to_le_bytes()
    }
}

#[repr(u8)]
pub enum GIOHash {
    Keccak256 = 2,
}

impl GIOHash {
    pub fn to_bytes(self) -> [u8; 1] {
        (self as u8).to_le_bytes()
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

#[derive(Clone)]
pub struct GIOClient {
    client: Client<HttpConnector>,
    url: Url,
}

impl GIOClient {
    pub fn new(url: Url) -> Self {
        let client = Client::new();
        Self { client, url }
    }

    pub async fn emit_gio(
        &self,
        domain: GIODomain,
        input: &Vec<u8>,
    ) -> Result<GIOResponse, GIOError> {
        // Encode request body
        let hex_data = hex::encode_prefixed(input);
        let request = GIOServerRequest {
            domain: domain as u32,
            id: hex_data,
        };
        let mut request_body = Vec::<u8>::new();
        serde_json::to_writer(&mut request_body, &request)
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;
        let request_body = Body::from(request_body);
        
        // Send request
        let request = Request::builder()
            .uri(self.url.to_string())
            .method("POST")
            .header("Content-Type", "application/json")
            .body(request_body)
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;
        let response = self.client
            .request(request)
            .await
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;
        
        if !response.status().is_success() {
            return Err(GIOError::EmitFailed(format!("response status code - {}", response.status())))
        }
        
        // Parse response
        let response_body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?; 
        let respones_json: GIOServerResponse = serde_json::from_slice(&response_body.to_vec())
            .map_err(|err| GIOError::EmitFailed(err.to_string()))?;
        let response_data =
            hex::decode(respones_json.response).map_err(|err| GIOError::EmitFailed(err.to_string()))?;
        
        Ok(GIOResponse {
            code: respones_json.response_code,
            data: response_data,
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
