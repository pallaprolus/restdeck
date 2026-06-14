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
    },
    Error(String),
}

pub fn run_request(
    url: String,
    method: String,
    headers_str: String,
    params_str: String,
    body_str: String,
    tx: Sender<HttpResponseEvent>,
) {
    tokio::spawn(async move {
        let _ = tx.send(HttpResponseEvent::Start).await;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        
        let client = match client {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(HttpResponseEvent::Error(format!("Failed to build HTTP client: {}", e))).await;
                return;
            }
        };

        // Construct URL with query parameters
        let mut final_url = url.trim().to_string();
        if !final_url.starts_with("http://") && !final_url.starts_with("https://") {
            final_url = format!("https://{}", final_url);
        }

        // Add query parameters
        let mut query_pairs = Vec::new();
        for line in params_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_string();
                let val = line[pos + 1..].trim().to_string();
                query_pairs.push((key, val));
            } else {
                // key without value
                query_pairs.push((line.to_string(), "".to_string()));
            }
        }

        let mut req_builder = match method.as_str() {
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

        // Add headers
        for line in headers_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(pos) = line.find(':') {
                let key = line[..pos].trim();
                let val = line[pos + 1..].trim();
                
                if let (Ok(h_key), Ok(h_val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(val),
                ) {
                    req_builder = req_builder.header(h_key, h_val);
                }
            }
        }

        // Add body for non-GET methods
        if method != "GET" && !body_str.is_empty() {
            req_builder = req_builder.body(body_str);
        }

        let start_time = Instant::now();
        let res = req_builder.send().await;

        match res {
            Ok(response) => {
                let duration = start_time.elapsed().as_millis();
                let status_str = format!("{} {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or(""));
                
                // Read response body as text
                match response.text().await {
                    Ok(text) => {
                        let size = text.len();
                        
                        // Attempt to pretty-print if it's JSON
                        let formatted_text = if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                            serde_json::to_string_pretty(&json_val).unwrap_or(text)
                        } else {
                            text
                        };

                        let _ = tx.send(HttpResponseEvent::Success {
                            body: formatted_text,
                            status: status_str,
                            time_ms: duration,
                            size_bytes: size,
                        }).await;
                    }
                    Err(e) => {
                        let _ = tx.send(HttpResponseEvent::Error(format!("Failed to read response body: {}", e))).await;
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(HttpResponseEvent::Error(format!("Network request failed:\n{}", e))).await;
            }
        }
    });
}
