use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::process::exit;
use std::thread;
use std::time::Duration;

use reqwest;
use serde::Deserialize;
use serde_json;
use tokio;

#[derive(Debug, Deserialize)]
struct Response {
    status_info: StatusInfo,
    data: Vec<DataItem>,
}

impl Response {
    fn is_status_ok(&self) -> bool {
        self.status_info.status_code == 200
    }

    fn is_unauthorized(&self) -> bool {
        self.status_info.status_code == 401
    }

    fn is_complete(&self) -> bool {
        if self.data.len() == 0 {
            return false;
        }
        self.data[0].did_start == true & self.data[0].did_complete
    }
}

#[derive(Debug, Deserialize)]
struct StatusInfo {
    status_code: u16,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DataItem {
    did_start: bool,
    did_complete: bool,
    current_state: String,
    // current_task: String,
}

struct APIClient {
    url: String,
    token: String,
}

impl APIClient {
    fn new(url: &str, token: &str) -> APIClient {
        APIClient {
            url: url.to_string(),
            token: token.to_string(),
        }
    }

    #[tokio::main]
    async fn status_check(&self) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/api/v1/task/status", self.url);

        let client = reqwest::Client::new();
        let mut token_header = reqwest::header::HeaderMap::new();
        token_header.insert(
            "Token",
            reqwest::header::HeaderValue::from_str(self.token.as_str()).unwrap(),
        );
        let mut waittime = 30;
        let mut last_state = String::new();
        loop {
            let headers = token_header.clone();

            let body = client
                .get(url.as_str())
                .headers(headers)
                .send()
                .await?
                .text()
                .await?;

            let response: Response =
                serde_json::from_str(&body).expect("response body in wrong format");

            if response.is_status_ok() {
                let current_state = response.data[0].current_state.clone();
                if last_state != current_state {
                    let headers = token_header.clone();
                    let mut map = HashMap::new();
                    map.insert("status", response.data[0].current_state.as_str());
                    client
                        .post(url.as_str())
                        .headers(headers)
                        .json(&map)
                        .send()
                        .await?;
                    last_state = current_state;
                }

                if !response.is_complete() {
                    println!("workflow is still running");
                    waittime = 30;
                } else {
                    println!("workflow is {}", response.data[0].current_state);
                    waittime = 600;
                }
            } else {
                println!("status query failed with {}", response.status_info.message);
                if response.is_unauthorized() {
                    // The token has expired and future calls will break
                    break;
                }
            }
            thread::sleep(Duration::from_secs(waittime));
        }
        Ok(())
    }
}

fn main() {
    let url = env::var("TFO_API_URL").expect("$TFO_API_URL is not set");
    let token = env::var("TFO_API_LOG_TOKEN").expect("$TFO_API_LOG_TOKEN is not set");
    let client = APIClient::new(url.as_str(), token.as_str());

    match client.status_check() {
        Ok(()) => exit(0),
        Err(err) => {
            println!("{}", err);
            exit(1)
        }
    };
}
