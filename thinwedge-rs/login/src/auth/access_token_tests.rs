use super::*;

#[test]
fn classifies_personal_access_tokens_by_prefix() {
    assert!(matches!(
        classify_thinwedge_access_token("at-example"),
        ThinWedgeAccessToken::PersonalAccessToken("at-example")
    ));
    assert!(matches!(
        classify_thinwedge_access_token("header.payload.signature"),
        ThinWedgeAccessToken::AgentIdentityJwt("header.payload.signature")
    ));
}
