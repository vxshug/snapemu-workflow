use std::collections::HashMap;

pub async fn log_down(eui: &str, token: &str, component: &str, tx: &str) {
    let mut m = HashMap::new();
    m.insert("eui", serde_json::Value::String(eui.to_string()));
    m.insert("token", serde_json::Value::String(token.to_string()));
    m.insert("component", serde_json::Value::String(component.to_string()));
    let json: Option<serde_json::Value> = serde_json::from_str(tx).ok();
    match json {
        None => {
            println!("Not serde json");
            return;
        }
        Some(json) => {
            m.insert("json", json);
        }
    }

    match std::env::var("DOWN_LOG").ok() {
        None => {
            println!("Not Found DOWN_LOG")
        }
        Some(down_url) => {
            if let Err(e) = _request(m, &down_url).await {
                println!("down log error: {}", e);
            }
        }
    }
}

async fn _request(body: HashMap<&str, serde_json::Value>, url: &str) -> Result<(), reqwest::Error> {
    reqwest::Client::builder().build()?.post(url).json(&body).send().await?;
    Ok(())
}
