use crate::app::ApiRequest;
use std::time::Instant;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum HttpResponseEvent {
    Start,
    Success {
        body: String,
        status: String,
        time_ms: u128,
        size_bytes: usize,

        // Pass back raw request details for history recording
        raw_url: String,
        raw_method: String,
        raw_headers: String,
        raw_params: String,
        raw_body: String,
    },
    Error {
        err: String,

        // Pass back raw request details for history recording
        raw_url: String,
        raw_method: String,
        raw_headers: String,
        raw_params: String,
        raw_body: String,
    },
}

pub fn create_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
}

pub fn parse_headers(headers_str: &str) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    for (line_idx, line) in headers_str.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(pos) = trimmed.find(':') else {
            return Err(format!(
                "Invalid header line {}: '{}' (expected 'Header-Name: value')",
                line_idx + 1,
                trimmed
            ));
        };
        let key = trimmed[..pos].trim();
        let val = trimmed[pos + 1..].trim();

        let h_name = reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
            format!(
                "Invalid header name on line {}: '{}' ({})",
                line_idx + 1,
                key,
                e
            )
        })?;
        let h_val = reqwest::header::HeaderValue::from_str(val).map_err(|e| {
            format!(
                "Invalid header value for '{}' on line {}: '{}' ({})",
                key,
                line_idx + 1,
                val,
                e
            )
        })?;
        headers.append(h_name, h_val);
    }
    Ok(headers)
}

pub fn run_request(
    client: reqwest::Client,
    req: ApiRequest,
    raw_req: ApiRequest,
    tx: Sender<HttpResponseEvent>,
) {
    tokio::spawn(async move {
        let _ = tx.send(HttpResponseEvent::Start).await;

        let headers = match parse_headers(&req.headers) {
            Ok(h) => h,
            Err(err) => {
                let _ = tx
                    .send(HttpResponseEvent::Error {
                        err: format!("Header Error:\n{}", err),
                        raw_url: raw_req.url,
                        raw_method: raw_req.method,
                        raw_headers: raw_req.headers,
                        raw_params: raw_req.params,
                        raw_body: raw_req.body,
                    })
                    .await;
                return;
            }
        };

        let mut final_url = req.url.trim().to_string();
        if !final_url.starts_with("http://") && !final_url.starts_with("https://") {
            final_url = format!("https://{}", final_url);
        }

        let query_pairs = parse_params(&req.params);

        let mut req_builder = match req.method.as_str() {
            "GET" => client.get(&final_url),
            "POST" => client.post(&final_url),
            "PUT" => client.put(&final_url),
            "DELETE" => client.delete(&final_url),
            "PATCH" => client.patch(&final_url),
            _ => client.get(&final_url),
        };

        if !query_pairs.is_empty() {
            req_builder = req_builder.query(&query_pairs);
        }

        req_builder = req_builder.headers(headers);

        if req.method != "GET" && !req.body.is_empty() {
            req_builder = req_builder.body(req.body);
        }

        let start_time = Instant::now();
        let res = req_builder.send().await;

        match res {
            Ok(response) => {
                let duration = start_time.elapsed().as_millis();
                let status_str = format!(
                    "{} {}",
                    response.status().as_u16(),
                    response.status().canonical_reason().unwrap_or("")
                );

                match response.text().await {
                    Ok(text) => {
                        let size = text.len();

                        let formatted_text = if let Ok(json_val) =
                            serde_json::from_str::<serde_json::Value>(&text)
                        {
                            serde_json::to_string_pretty(&json_val).unwrap_or(text)
                        } else {
                            text
                        };

                        let _ = tx
                            .send(HttpResponseEvent::Success {
                                body: formatted_text,
                                status: status_str,
                                time_ms: duration,
                                size_bytes: size,
                                raw_url: raw_req.url,
                                raw_method: raw_req.method,
                                raw_headers: raw_req.headers,
                                raw_params: raw_req.params,
                                raw_body: raw_req.body,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(HttpResponseEvent::Error {
                                err: format!("Failed to read response body: {}", e),
                                raw_url: raw_req.url,
                                raw_method: raw_req.method,
                                raw_headers: raw_req.headers,
                                raw_params: raw_req.params,
                                raw_body: raw_req.body,
                            })
                            .await;
                    }
                }
            }
            Err(e) => {
                let _ = tx
                    .send(HttpResponseEvent::Error {
                        err: format!("Network request failed:\n{}", e),
                        raw_url: raw_req.url,
                        raw_method: raw_req.method,
                        raw_headers: raw_req.headers,
                        raw_params: raw_req.params,
                        raw_body: raw_req.body,
                    })
                    .await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_headers() {
        let headers_str =
            "Content-Type: application/json\nAuthorization: Bearer token123\nAccept: text/plain\n";
        let headers = parse_headers(headers_str).expect("Valid headers should parse");
        assert_eq!(headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(headers.get("Authorization").unwrap(), "Bearer token123");
        assert_eq!(headers.get("Accept").unwrap(), "text/plain");
    }

    #[test]
    fn test_parse_headers_missing_colon() {
        let headers_str = "Content-Type: application/json\nInvalidHeaderWithoutColon";
        let err = parse_headers(headers_str).unwrap_err();
        assert!(err.contains("expected 'Header-Name: value'"));
        assert!(err.contains("line 2"));
    }

    #[test]
    fn test_parse_headers_invalid_name() {
        let headers_str = "Invalid Header Name: value";
        let err = parse_headers(headers_str).unwrap_err();
        assert!(err.contains("Invalid header name on line 1"));
    }

    #[test]
    fn test_parse_headers_invalid_value() {
        let headers_str = "X-Test: value\nwith\nnewlines";
        // Each line is processed; "with" has no colon
        let err = parse_headers(headers_str).unwrap_err();
        assert!(err.contains("expected 'Header-Name: value'"));
    }

    #[test]
    fn test_parse_headers_empty_and_whitespace() {
        let headers_str = "  \n\n  Accept: */*  \n   \n";
        let headers = parse_headers(headers_str).expect("Empty lines should be ignored");
        assert_eq!(headers.get("Accept").unwrap(), "*/*");
    }
}

pub(crate) fn parse_params(s: &str) -> Vec<(String, String)> {
    let mut query_pairs = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let val = line[pos + 1..].trim().to_string();
            query_pairs.push((key, val));
        } else {
            query_pairs.push((line.to_string(), "".to_string()));
        }
    }
    query_pairs
}

#[cfg(test)]
mod param_tests {
    use super::*;

    #[test]
    fn test_parse_params() {
        let input = "
        
  key1=value1  
key2 = value2
key3=value=with=equals
key_no_value
";
        let params = parse_params(input);
        assert_eq!(params.len(), 4);
        assert_eq!(params[0], ("key1".to_string(), "value1".to_string()));
        assert_eq!(params[1], ("key2".to_string(), "value2".to_string()));
        assert_eq!(params[2], ("key3".to_string(), "value=with=equals".to_string()));
        assert_eq!(params[3], ("key_no_value".to_string(), "".to_string()));
    }
}
