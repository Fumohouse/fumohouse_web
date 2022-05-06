use crate::routes::BaseData;
use comrak::{nodes::NodeValue, Arena, ComrakExtensionOptions, ComrakOptions};
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::Path, str};

const FRONT_MATTER_SEP: &str = "---";

#[derive(Serialize, Deserialize)]
pub struct FrontMatter {
    pub category: String,
    pub page: Option<String>,
    pub title: String,
}

quick_error! {
    #[derive(Debug)]
    pub enum MarkdownError {
        NoFrontMatter {
            display("No front matter was found.")
        }
    }
}

#[derive(Serialize)]
struct MarkdownContext<'a> {
    base: BaseData<'a>,
    html: &'a str,
    front_matter: FrontMatter,
}

macro_rules! markdown_route {
    ($route_name:ident, $route:literal, $file_name:literal) => {
        #[get($route)]
        fn $route_name(
            csrf: crate::util::CsrfToken,
            user_session: crate::util::UserSession,
        ) -> Result<rocket_dyn_templates::Template, rocket::http::Status> {
            let result = crate::util::markdown::template(
                $file_name,
                crate::routes::BaseData {
                    user: user_session.user,
                    csrf_token: &csrf.token,
                },
            );

            match result {
                Ok(template) => Ok(template),
                Err(err) => {
                    error!("error parsing markdown: {}", err);
                    Err(rocket::http::Status::InternalServerError)
                }
            }
        }
    };
}

pub(crate) use markdown_route;

pub fn template<'a>(file_name: &str, base_data: BaseData<'a>) -> Result<Template, Box<dyn Error>> {
    let path = Path::new("markdown").join(file_name);
    let contents = fs::read_to_string(path)?;

    let arena = Arena::new();
    let root = comrak::parse_document(
        &arena,
        &contents,
        &ComrakOptions {
            extension: ComrakExtensionOptions {
                front_matter_delimiter: Some(FRONT_MATTER_SEP.to_string()),
                ..Default::default()
            },
            ..Default::default()
        },
    );

    let front_matter = root
        .children()
        .find_map(|child| match &child.data.borrow().value {
            NodeValue::FrontMatter(buffer) => Some(buffer.clone()),
            _ => None,
        })
        .ok_or(MarkdownError::NoFrontMatter)?;

    let front_matter = str::from_utf8(&front_matter)?
        .lines()
        // Should be fine, since valid front matter will only have
        // the separator at the first and last lines
        .filter(|l| *l != FRONT_MATTER_SEP)
        .collect::<Vec<&str>>()
        .join("\n");

    let front_matter: FrontMatter = serde_yaml::from_str(&front_matter)?;

    let mut html = Vec::new();
    comrak::format_html(root, &ComrakOptions::default(), &mut html)?;
    let html = str::from_utf8(&html)?;

    Ok(Template::render(
        "markdown",
        MarkdownContext {
            base: base_data,
            html: &html,
            front_matter,
        },
    ))
}
