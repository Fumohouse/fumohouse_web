use rand::{distributions::Alphanumeric, Rng};
use sha2::{Digest, Sha256};

mod captcha;
mod csrf;
mod session;
mod messages;

pub use captcha::CaptchaVerifier;

pub use csrf::CsrfToken;
pub use csrf::CsrfVerify;

pub use session::SessionUtils;

pub use messages::SiteMessages;

fn rand_string(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>()
}

fn sha256(input: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());

    hasher.finalize().as_slice().into()
}
