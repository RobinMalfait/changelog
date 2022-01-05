use reqwest::header::{HeaderValue, CONTENT_TYPE, USER_AGENT};

pub fn graphql(data: serde_json::Value) -> Result<serde_json::Value, String> {
    let json = reqwest::blocking::Client::new()
        .post("https://api.github.com/graphql")
        .bearer_auth(std::env::var("GITHUB_API_TOKEN").expect("GITHUB_API_TOKEN not set"))
        .header(USER_AGENT, HeaderValue::from_static("reqwest"))
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(data.to_string())
        .send()
        .unwrap()
        .json::<serde_json::Value>()
        .unwrap();

    if let Some(errors) = json["errors"].as_array() {
        return Err(errors[0]["message"].as_str().unwrap().to_string());
    }

    Ok(json)
}
