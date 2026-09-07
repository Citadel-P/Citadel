use citadel_domain::WebhookActivitySource;
use serde_json::Value;

// Only authenticated, bounded identifiers are kept. Never persist clone URLs,
// arbitrary payloads or authentication headers as activity metadata.
pub(super) fn source(headers: &[(String, String)], body: &[u8]) -> WebhookActivitySource {
    let payload: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let header = |names: &[&str]| {
        names.iter().find_map(|name| {
            headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        })
    };
    let commit = string(
        payload
            .get("after")
            .or_else(|| payload.get("checkout_sha"))
            .or_else(|| payload.get("commitSha"))
            .or_else(|| payload.get("commit")),
    )
    .filter(|sha| matches!(sha.len(), 40 | 64) && sha.bytes().all(|b| b.is_ascii_hexdigit()));
    WebhookActivitySource {
        delivery_id: identifier(header(&[
            "x-github-delivery",
            "x-gitlab-event-uuid",
            "x-gitea-delivery",
            "x-forgejo-delivery",
            "webhook-id",
        ])),
        event_type: identifier(header(&[
            "x-github-event",
            "x-gitlab-event",
            "x-gitea-event",
            "x-forgejo-event",
        ])),
        branch: identifier(
            string(payload.get("ref").or_else(|| payload.get("branch")))
                .map(|branch| branch.strip_prefix("refs/heads/").unwrap_or(branch)),
        ),
        commit_sha: commit.map(str::to_owned),
        repository_full_name: identifier(string(
            payload
                .pointer("/repository/full_name")
                .or_else(|| payload.pointer("/project/path_with_namespace")),
        )),
    }
}

fn string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

fn identifier(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 256
                && value
                    .chars()
                    .all(|c| c.is_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ' '))
        })
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_metadata_is_bounded_and_never_uses_clone_urls_or_credentials() {
        let payload = serde_json::json!({"ref":"refs/heads/main","after":"a".repeat(40),"repository":{"full_name":"team/project","clone_url":"https://user:secret@host/project"}});
        let source = source(
            &[
                ("x-forgejo-delivery".into(), "delivery-1".into()),
                ("authorization".into(), "Bearer secret".into()),
            ],
            &serde_json::to_vec(&payload).unwrap(),
        );
        assert_eq!(source.branch.as_deref(), Some("main"));
        assert_eq!(source.repository_full_name.as_deref(), Some("team/project"));
        assert_eq!(source.delivery_id.as_deref(), Some("delivery-1"));
        assert!(!serde_json::to_string(&source).unwrap().contains("secret"));
        assert!(identifier(Some(&"x".repeat(257))).is_none());
        assert!(identifier(Some("https://user:secret@host")).is_none());
    }
}
