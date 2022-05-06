use crate::util::markdown::markdown_route;
use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![index, policy, code, contributor_agreement]
}

markdown_route!(index, "/", "home.md");

markdown_route!(policy, "/policy", "policy/policy.md");
markdown_route!(code, "/policy/code", "policy/code_of_conduct.md");
markdown_route!(
    contributor_agreement,
    "/policy/contributors",
    "policy/contributor_agreement.md"
);
