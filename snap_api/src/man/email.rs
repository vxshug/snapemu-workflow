use crate::error::{ApiError, ApiResult};
use crate::load::load_config;
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use rust_embed::RustEmbed;
use std::str::FromStr;
use tracing::warn;

#[derive(RustEmbed)]
#[include = "*.html"]
#[folder = "resources/"]
struct EmailTemplate;
pub struct EmailManager {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    sender: Mailbox,
}

impl EmailManager {
    pub(crate) fn new() -> ApiResult<Option<Self>> {
        let config = load_config();
        if let Some(email) = config.api.email.as_ref() {
            let url = format!("smtps://{}:{}", email.server, email.port);
            let credential = lettre::transport::smtp::authentication::Credentials::new(
                email.user.clone(),
                email.password.clone(),
            );
            let mailer: AsyncSmtpTransport<Tokio1Executor> =
                AsyncSmtpTransport::<Tokio1Executor>::from_url(url.as_str())
                    .map_err(|e| ApiError::Server {
                        case: "email config",
                        msg: e.to_string().into(),
                    })?
                    .credentials(credential)
                    .build();
            let sender = Mailbox::new(
                Some("norplay".to_string()),
                lettre::address::Address::from_str(&email.user).map_err(|_e| ApiError::Server {
                    case: "user email",
                    msg: "invalid email address".into(),
                })?,
            );
            return Ok(Some(Self { mailer, sender }));
        }
        Ok(None)
    }
}

impl EmailManager {
    fn load_template(file: &str) -> ApiResult<String> {
        let index_html =
            EmailTemplate::get(file).ok_or(ApiError::User("not found email template".into()))?;
        let sign = std::str::from_utf8(index_html.data.as_ref())
            .map_err(|_| ApiError::User("parse email template error".into()))?;
        Ok(sign.to_string())
    }

    async fn send(&self, username: &str, email: &str, page: String, subject: &str) -> ApiResult {
        let receiver = format!("{} <{}>", username, email)
            .parse()
            .map_err(|_| ApiError::User(format!("email format error: {}", email).into()))?;
        let email = Message::builder()
            .from(self.sender.clone())
            .to(receiver)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(page)
            .map_err(|e| {
                warn!("send email error: {}", e);
                ApiError::User("send email error".into())
            })?;
        self.mailer.send(email).await.map_err(|e| {
            warn!("send email error: {}", e);
            ApiError::User("send email error".into())
        })?;
        Ok(())
    }
    pub async fn sign(&self, username: &str, email: &str, call_url: &str) -> ApiResult {
        let sign = Self::load_template("sign.zh.html")?;

        let page = sign.replace("useremailurl", call_url).replace("username", username);
        self.send(username, email, page, "Verify Email").await
    }

    pub async fn valid_code(&self, username: &str, email: &str, code: &str) -> ApiResult {
        let sign = Self::load_template("valicode.html")?;
        let page = sign.replace("[Recipient Name]", username).replace("[CODE]", code);
        self.send(username, email, page, "Verify Email").await
    }

    pub async fn delete_valid_code(&self, username: &str, email: &str, code: &str) -> ApiResult {
        let sign = Self::load_template("delete.html")?;
        let page = sign.replace("[Verification Code]", code);
        self.send(username, email, page, "Verify Email").await
    }
    pub async fn contact(&self, username: &str, email: &str, message: &str) -> ApiResult {
        let contact_template = Self::load_template("contact.html")?;
        let page = contact_template
            .replace("[Name]", username)
            .replace("[Email]", email)
            .replace("[Message]", message);
        let config = load_config();
        let email = config
            .concat_email
            .clone()
            .ok_or_else(|| ApiError::User("no contact email configuration found".into()))?;
        self.send("Contact", &email, page, "Contact Email").await
    }
}
