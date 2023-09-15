use std::env;
use std::error::Error;
use std::thread;
use std::time::Duration;

use reqwest;
use serde::Deserialize;
use serde_json;
use tokio;

// Query status of this resource (cluster/namespace/name)
// Parse the json response of status
// Fail if not status 200
// On data, complete on wf completion with status report
// Restart policy shall be OnFailure?
// TTL should be 0 to clean up environment quickly?

#[derive(Debug, Deserialize)]
struct Response {
    status_info: StatusInfo,
    data: Vec<DataItem>,
}

impl Response {
    fn is_status_ok(&self) -> bool {
        self.status_info.status_code == 200
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
    current_task: String,
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
    async fn status_check(&self) -> Result<&str, Box<dyn Error>> {
        let url = format!("{}/api/v1/task/status", self.url);

        let client = reqwest::Client::new();

        loop {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Token",
                reqwest::header::HeaderValue::from_str(self.token.as_str()).unwrap(),
            );

            let body = client
                .get(url.as_str())
                .headers(headers)
                .send()
                .await?
                .text()
                .await?;

            let response: Response =
                serde_json::from_str(&body).expect("response body in wrong format");

            if response.is_status_ok() & response.is_complete() {
                // send a query to update status in database
                println!("{}", response.data[0].current_state);
                break;
            } else if !response.is_status_ok() {
                println!("{}", response.status_info.message);
            } else if !response.is_complete() {
                println!("workflow is still running");
            }

            thread::sleep(Duration::from_secs(30));
        }

        Ok((""))
    }
}

fn main() {
    let url = env::var("TFO_API_URL").expect("$TFO_API_URL is not set");
    let token = env::var("TFO_API_LOG_TOKEN").expect("$TFO_API_LOG_TOKEN is not set");
    let client = APIClient::new(url.as_str(), token.as_str());

    let response = client.status_check();
}
