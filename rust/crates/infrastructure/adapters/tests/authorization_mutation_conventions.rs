//! ACL SQL must stay in audited owners. New writers require an explicit review of
//! pre/post impact capture and commit publication, not just a new cache TTL.
use std::{collections::BTreeSet, path::Path};

#[test]
fn authorization_writers_use_the_committed_mutation_boundary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/persistence/postgres");
    let audited: BTreeSet<_> = [
        "identity/actors/repository.rs",
        "identity/authentication/store.rs",
        "identity/oidc/store.rs",
        "identity/roles/repository.rs",
        "identity/service_accounts/repository.rs",
        "identity/teams/repository.rs",
        "identity/users/repository.rs",
        "platforms/deletion.rs",
        "deployments/claims.rs",
    ]
    .into_iter()
    .collect();
    let mut found = BTreeSet::new();
    visit(&root, &root, &mut found);
    assert_eq!(
        found.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        audited,
        "Review new ACL SQL owners and their impact capture"
    );
    // Remaining plain commits are deliberately limited to read-only, metadata,
    // session and token methods. A new transaction is not silently exempt.
    let exceptions = [
        "get",
        "rename",
        "create_session",
        "delete_owned_session",
        "delete_other_sessions",
        "create_token",
        "revoke_token",
        "create",
        "update",
        "update_description",
        "delete",
        "start_login",
        "claim_delete_impl",
        "release_delete_impl",
        "claim_apply_versioned_impl",
        "complete_apply_impl",
        "fail_apply_impl",
    ];
    for file in &audited {
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        assert!(source.contains("Mutation::enter(&self.pool)"), "{file}");
        for part in source.split("fn ").skip(1) {
            if !part.contains("self.pool.begin()") {
                continue;
            }
            let name = part.split(['(', '<']).next().unwrap().trim();
            if part.contains("Mutation::enter") {
                assert!(
                    part.contains("authorization") && part.contains(".commit("),
                    "{file}: {name}"
                );
                assert!(
                    !part.contains("transaction.commit()") && !part.contains("tx.commit()"),
                    "{file}: {name} bypasses invalidation"
                );
            } else {
                assert!(
                    exceptions.contains(&name),
                    "Audit {file}: {name} before allowing a plain commit"
                );
                // These generic names are exempt only in these specific owners.
                if [
                    "create",
                    "update",
                    "update_description",
                    "delete",
                    "start_login",
                ]
                .contains(&name)
                {
                    assert_eq!(*file, "identity/oidc/store.rs");
                }
            }
        }
    }
}
fn visit(root: &Path, dir: &Path, found: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit(root, &path, found);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap().to_lowercase();
        let tokens: Vec<_> = source
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|t| !t.is_empty())
            .collect();
        let tables = [
            "actors",
            "actorroles",
            "permissions",
            "resourceaccesses",
            "actorteammemberships",
            "teams",
            "roles",
        ];
        if tokens.windows(3).any(|w| {
            (w[0] == "insert" && w[1] == "into" || w[0] == "delete" && w[1] == "from")
                && tables.contains(&w[2])
        }) || tokens
            .windows(2)
            .any(|w| w[0] == "update" && tables.contains(&w[1]))
        {
            found.insert(path.strip_prefix(root).unwrap().to_str().unwrap().into());
        }
    }
}
