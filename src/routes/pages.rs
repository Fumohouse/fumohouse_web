use crate::util::markdown::markdown_route;
use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![index, policy, code, contributor_agreement]
}

markdown_route!(index, "/", "home.md");

markdown_route!(policy, "/rules", "rules/rules.md");
markdown_route!(code, "/rules/code", "rules/code_of_conduct.md");
markdown_route!(
    contributor_agreement,
    "/rules/contributors",
    "rules/contributor_agreement.md"
);
