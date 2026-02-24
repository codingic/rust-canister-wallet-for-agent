use ic_cdk::management_canister::{
    self, HttpHeader, HttpMethod, HttpRequestArgs, HttpRequestResult,
};

use crate::error::{WalletError, WalletResult};

pub async fn http_request(args: &HttpRequestArgs, op: &str) -> WalletResult<HttpRequestResult> {
    management_canister::http_request(args)
        .await
        .map_err(|err| WalletError::Internal(format!("{op} http outcall failed: {err}")))
}

pub async fn json_request(
    url: String,
    method: HttpMethod,
    body: Option<Vec<u8>>,
    max_response_bytes: u64,
    op: &str,
) -> WalletResult<HttpRequestResult> {
    let has_body = body.is_some();
    let mut headers = vec![HttpHeader {
        name: "accept".to_string(),
        value: "application/json".to_string(),
    }];
    if has_body {
        headers.push(HttpHeader {
            name: "content-type".to_string(),
            value: "application/json".to_string(),
        });
    }
    let args = HttpRequestArgs {
        url,
        max_response_bytes: Some(max_response_bytes),
        method,
        headers,
        body,
        transform: None,
    };
    http_request(&args, op).await
}

pub async fn get_json(
    url: String,
    max_response_bytes: u64,
    op: &str,
) -> WalletResult<HttpRequestResult> {
    json_request(url, HttpMethod::GET, None, max_response_bytes, op).await
}

pub async fn post_json(
    url: String,
    body: Vec<u8>,
    max_response_bytes: u64,
    op: &str,
) -> WalletResult<HttpRequestResult> {
    json_request(url, HttpMethod::POST, Some(body), max_response_bytes, op).await
}

pub async fn post_text(
    url: String,
    body: Vec<u8>,
    content_type: &str,
    accept: &str,
    max_response_bytes: u64,
    op: &str,
) -> WalletResult<HttpRequestResult> {
    let args = HttpRequestArgs {
        url,
        max_response_bytes: Some(max_response_bytes),
        method: HttpMethod::POST,
        headers: vec![
            HttpHeader {
                name: "content-type".to_string(),
                value: content_type.to_string(),
            },
            HttpHeader {
                name: "accept".to_string(),
                value: accept.to_string(),
            },
        ],
        body: Some(body),
        transform: None,
    };
    http_request(&args, op).await
}
