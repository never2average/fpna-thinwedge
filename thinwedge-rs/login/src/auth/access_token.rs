const PERSONAL_ACCESS_TOKEN_PREFIX: &str = "at-";

pub(super) enum ThinWedgeAccessToken<'a> {
    PersonalAccessToken(&'a str),
    AgentIdentityJwt(&'a str),
}

pub(super) fn classify_thinwedge_access_token(access_token: &str) -> ThinWedgeAccessToken<'_> {
    if access_token.starts_with(PERSONAL_ACCESS_TOKEN_PREFIX) {
        ThinWedgeAccessToken::PersonalAccessToken(access_token)
    } else {
        ThinWedgeAccessToken::AgentIdentityJwt(access_token)
    }
}

#[cfg(test)]
#[path = "access_token_tests.rs"]
mod tests;
