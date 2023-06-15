use candid::CandidType;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

type HeaderField = (String, String);

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct CandidHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}

#[derive(Clone, Debug, CandidType, Serialize, Deserialize)]
pub struct CandidHttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    pub body: ByteBuf,
}
