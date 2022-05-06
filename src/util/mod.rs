use argon2::{
    password_hash::{Error as ArgonError, SaltString},
    Argon2, PasswordHasher,
};
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch, InitError,
};
use log::LevelFilter;
use rand::{distributions::Alphanumeric, rngs::OsRng, Rng};
use sha2::{Digest, Sha256};

mod captcha;
mod csrf;
pub mod markdown;
mod messages;
mod session;

pub use captcha::CaptchaVerifier;

pub use csrf::CsrfToken;
pub use csrf::CsrfVerify;

pub use session::{SessionUtils, UserSession};

pub use messages::SiteMessages;

pub fn setup_logging(debug: bool) -> Result<(), InitError> {
    let colors = ColoredLevelConfig::new()
        .debug(Color::Green)
        .info(Color::Blue)
        .warn(Color::Yellow)
        .error(Color::Red)
        .trace(Color::Magenta);

    Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} > {} > {} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                colors.color(record.level()),
                message
            ))
        })
        .level(if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        })
        .chain(std::io::stdout())
        .apply()?;

    Ok(())
}

pub fn hash_password(argon: &Argon2, password: &str) -> Result<String, ArgonError> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed_pass = argon.hash_password(password.as_bytes(), &salt)?;

    Ok(hashed_pass.to_string())
}

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
