use rocket::form::Error as FormError;

pub enum SiteMessages {
    // TODO: Wherever GenericError is used, details should be logged
    GenericError,
    CAPTCHAFailed,
    UsernameInUse,
    UsernameInvalid,
    LoginFailed,
    PasswordIncorrect,
    PasswordsDontMatch,
}

impl SiteMessages {
    pub fn description(&self) -> &str {
        match self {
            Self::GenericError => "An internal error occurred. Please try again or contact the site admin.",
            Self::CAPTCHAFailed => "Invalid CAPTCHA response.",
            Self::UsernameInvalid => "Username contains invalid characters.",
            Self::UsernameInUse => "Username is in use.",
            Self::LoginFailed => "Invalid username/password.",
            Self::PasswordIncorrect => "Password is incorrect.",
            Self::PasswordsDontMatch => "Passwords don't match.",
        }
    }

    pub fn field_name(&self) -> Option<&'static str> {
        match self {
            Self::UsernameInUse => Some("username"),
            Self::PasswordIncorrect => Some("current_password"),
            Self::PasswordsDontMatch => Some("verify_password"),
            _ => None,
        }
    }
}

impl ToString for SiteMessages {
    fn to_string(&self) -> String {
        self.description().to_string()
    }
}

impl<'a> Into<FormError<'a>> for SiteMessages {
    fn into(self) -> FormError<'a> {
        let e = FormError::validation(self.to_string());

        if let Some(field_name) = self.field_name() {
            return e.with_name(field_name);
        }

        e
    }
}