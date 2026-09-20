use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{Duration, Utc};
use citadel_adapters::oidc_protocol::OidcHttpProtocol;
use citadel_identity::OidcProtocol;
use citadel_identity::{OidcProvider, SYSTEM_ACTOR_ID};
use citadel_primitives::ActorId;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};

const PRIVATE_KEY_DER: &str = "MIIEowIBAAKCAQEAiRxt1FHRTs1on3ut1Vuhg1RD/g8cnu2l76ACeImyB8eOQkCWWF3pWDGkg7vZOMC3VFkz/exIXVFDb1g7bP7EaNiSbEs9p0XCmaYblfccPtD9RvkeSePCLRN6dWBeLeulksnqiLBrf3O+6F10fHbT8PT7tLb9sypURGF01mB42w9WJqBN9aDkPyvZS8wp7BeE0x8hd7zpR/HyT96/njf1VSpQU6OkzkF4+efz3x5gmsP81AKVq4fKQervz58K+IqZpf7NItTx8K0a+jc5IM8jRzSL5LNNev5oF4I0foZXPzWlH08gOIk+nr6ltNw+wh4xLwf1dSqDlM0uMO2kQZzZPwIDAQABAoIBACZXEscqUdMtUTI3jXZ59wIYUCL86s3uOlZ1cftu1Z+jR75y/RecuyF0UEKeBrH+AcXOY+F/bwTZMngyPfvOifGfjeJGb+kUcsQwVMpsnTNbkVeFVdjnnWapabbkybEhkd0oJTMv0f+DyECF0Yr0V3Orra7s9KhjD8lHTzqbI3BtwqYp4wMNIPZaMHVtHlxX29YqpFo4Fhlx1CoILsieZ3A+/9kvUMHrxnV77vdWL/OWJuMYiGevBgZQ38yPIvJa+sy+X3BKUUjLSmNesypd9iZ3f3HMPzJi08bRRRPgeuZ5laEcSFd+ejft3DiMXbRGsdsWhRQtLtvFFsHhRFB3XrECgYEAv08ZzuwP01L+aqwrtUbW5i9wPqUEvvBJrkfgh3WpNPV7lgreF93/GMYntwdoQ5hen8Uue/c/m2chFCMX6qJ170mMpmiwqzx+akWL3Bk2+GzlKi5nrE2TH1DQ9UpQYLhWK17ZvSvBGRZF8zC/2oMDosyewTSQWTDcV/XwwrKjGFMCgYEAt3md5GmzbiHqOpRfnQ1R8GMOCYF/Bt38gXEe3jafO/4IEtGR68zalEMHHHjfRs71twCWh/JB5qSacegp0ZzgtjX8eNQ2MNAARJpHwV6ihnwibhIbALfTaJpeDDNEXk4KFIyM1b2JdO2w/b+juM6lWkEaYy3hOf6i/jJh51+YreUCgYBAyhYweMvnQr08/TAURXh0Hm7CGHrh+1jIuDj3R0bV06lWKDoNFTbWeg1rNAwAkHLDYzEZ9KNLCwMt4bCw2vJb5qnGlb/3ThZ2ATWbhcKTIbX+shaUSPVhbqpF2DQefW9ZYtcU9OOBjoSEFudyplot4WSGhDm6qwyEkZtHJN6NVwKBgBECmXzfv97qDgk4UFUDMyGSTW7cLqa4Vfy6PB5l+gVZ2+3CuECgUXOFc1dUbX7nGSjKSSp5b1qu0BLXb2kbnknGX43kPtHvttalZxmqaG8HezmxPAepA1SjWyzOY/xKR+z3yubWF7RbhRlBdzBfGD1x47xmOIdj8ECY8zK8Ti9xAoGBAIWlvzfNYCFDRGb1j1V1gCg+z4viQm0u1+IiPzNEfeooOSR3Ni7Mlt+aystM+pc1eUlsq9q+xyMp1Iu7FYEEUQOclld2hJMmbEdxgXbCCjv+szCmaMkXPJNf5ZA0CCcw5WTisSoViufiagHwhDi2vcUBL2wJh1wg7gTvKRDDzOu3";
const MODULUS: &str = "iRxt1FHRTs1on3ut1Vuhg1RD_g8cnu2l76ACeImyB8eOQkCWWF3pWDGkg7vZOMC3VFkz_exIXVFDb1g7bP7EaNiSbEs9p0XCmaYblfccPtD9RvkeSePCLRN6dWBeLeulksnqiLBrf3O-6F10fHbT8PT7tLb9sypURGF01mB42w9WJqBN9aDkPyvZS8wp7BeE0x8hd7zpR_HyT96_njf1VSpQU6OkzkF4-efz3x5gmsP81AKVq4fKQervz58K-IqZpf7NItTx8K0a-jc5IM8jRzSL5LNNev5oF4I0foZXPzWlH08gOIk-nr6ltNw-wh4xLwf1dSqDlM0uMO2kQZzZPw";

#[derive(Clone)]
struct IssuerState {
    issuer: Arc<String>,
}

#[tokio::test]
async fn protocol_discovers_exchanges_and_validates_a_signed_id_token() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = Arc::new(format!("http://{}", listener.local_addr().unwrap()));
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/token", post(token))
        .route("/jwks", get(jwks))
        .with_state(IssuerState {
            issuer: issuer.clone(),
        });
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let protocol = OidcHttpProtocol::new(std::time::Duration::from_secs(5)).unwrap();
    let discovery = protocol.discover(&issuer).await.unwrap();
    assert_eq!(discovery.issuer, *issuer);
    let provider = provider(&issuer);
    let identity = protocol
        .exchange_and_validate(
            &provider,
            &discovery,
            "client-secret",
            "authorization-code",
            "http://citadel.test/api/v1/authentication/oidc/callback",
            "pkce-verifier",
            "expected-nonce",
        )
        .await
        .unwrap();
    assert_eq!(identity.subject, "subject-1");
    assert_eq!(identity.email.as_deref(), Some("person@example.test"));
    assert!(identity.email_verified);
    assert_eq!(
        identity.claims.get("groups"),
        Some(&vec!["operators".to_owned(), "readers".to_owned()])
    );

    let rejected = protocol
        .exchange_and_validate(
            &provider,
            &discovery,
            "client-secret",
            "authorization-code",
            "http://citadel.test/api/v1/authentication/oidc/callback",
            "pkce-verifier",
            "wrong-nonce",
        )
        .await;
    assert!(rejected.is_err());
    server.abort();
}

async fn discovery(State(state): State<IssuerState>) -> Json<Value> {
    Json(json!({
        "issuer": state.issuer.as_str(),
        "authorization_endpoint": format!("{}/authorize", state.issuer),
        "token_endpoint": format!("{}/token", state.issuer),
        "jwks_uri": format!("{}/jwks", state.issuer)
    }))
}

async fn jwks() -> Json<Value> {
    Json(json!({
        "keys": [{
            "kty": "RSA",
            "kid": "test-signing-key",
            "use": "sig",
            "alg": "RS256",
            "n": MODULUS,
            "e": "AQAB"
        }]
    }))
}

async fn token(
    State(state): State<IssuerState>,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    assert_eq!(
        form.get("grant_type").map(String::as_str),
        Some("authorization_code")
    );
    assert_eq!(
        form.get("client_id").map(String::as_str),
        Some("citadel-client")
    );
    assert_eq!(
        form.get("code_verifier").map(String::as_str),
        Some("pkce-verifier")
    );
    let now = Utc::now();
    let claims = json!({
        "iss": state.issuer.as_str(),
        "aud": "citadel-client",
        "sub": "subject-1",
        "exp": (now + Duration::minutes(5)).timestamp(),
        "iat": now.timestamp(),
        "nonce": "expected-nonce",
        "email": "person@example.test",
        "email_verified": true,
        "preferred_username": "person",
        "groups": ["operators", "readers"]
    });
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-signing-key".to_owned());
    let key = EncodingKey::from_rsa_der(&STANDARD.decode(PRIVATE_KEY_DER).unwrap());
    Json(json!({ "id_token": encode(&header, &claims, &key).unwrap() }))
}

fn provider(issuer: &str) -> OidcProvider {
    OidcProvider::new(
        "test-provider".to_owned(),
        None,
        "Test Provider".to_owned(),
        issuer.to_owned(),
        "citadel-client".to_owned(),
        None,
        "openid profile email".to_owned(),
        true,
        false,
        false,
        true,
        None,
        None,
        None,
        None,
        ActorId::new(SYSTEM_ACTOR_ID),
        Utc::now(),
    )
    .unwrap()
}
