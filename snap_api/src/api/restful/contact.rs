use crate::api::SnJson;
use crate::error::ApiResponseResult;
use crate::man::EmailManager;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Message {
    name: String,
    email: String,
    message: String,
}

pub async fn contact_us(SnJson(message): SnJson<Message>) -> ApiResponseResult<String> {
    let email_manager = EmailManager::new()?;
    if let Some(email_manager) = email_manager {
        email_manager.contact(&message.name, &message.email, &message.message).await?;
    }
    Ok("Ok".to_string().into())
}
