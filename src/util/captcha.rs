use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{env, error::Error};

const HCAPTCHA_URL: &str = "https://hcaptcha.com/siteverify";

#[derive(Serialize)]
struct VerifyBody<'a> {
    response: &'a str,
    secret: &'a str,
}

#[derive(Deserialize)]
struct VerifyResponse {
    success: bool,
}

pub struct CaptchaVerifier {
    client: Client,
    pub site_key: String,
    secret: String,
}

impl CaptchaVerifier {
    pub fn new() -> CaptchaVerifier {
        let client = Client::builder()
            .build()
            .expect("Failed to create CAPTCHA client");

        CaptchaVerifier {
            client,
            site_key: env::var("HCAPTCHA_SITEKEY").expect("Did not find HCAPTCHA_SITE_KEY."),
            secret: env::var("HCAPTCHA_SECRET").expect("Did not find HCAPTCHA_SECRET."),
        }
    }

    pub async fn verify(&self, client_response: &str) -> Result<bool, Box<dyn Error>> {
        let body = VerifyBody {
            response: client_response,
            secret: &self.secret,
        };

        let res = self.client.post(HCAPTCHA_URL).form(&body).send().await?;
        let data = res.json::<VerifyResponse>().await?;

        Ok(data.success)
    }
}
