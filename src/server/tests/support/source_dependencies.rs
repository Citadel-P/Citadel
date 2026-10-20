//! Small source guard: parse imports/paths, including grouped and renamed imports.
//! Inspect macro bodies too, since orchestration often sits inside `select!`.
use syn::visit::{self, Visit};

#[derive(Default)]
pub struct Dependencies {
    paths: Vec<String>,
    aliases: Vec<(String, String)>,
    methods: Vec<String>,
}

impl Dependencies {
    pub fn parse(source: &str) -> Self {
        let mut dependencies = Self::default();
        dependencies.visit_file(&syn::parse_file(source).expect("valid Rust source"));
        // Resolve ordinary import aliases, including namespace aliases. This is
        // deliberately conservative, not compiler-level scope/name resolution.
        let aliases: std::collections::HashMap<_, _> =
            dependencies.aliases.iter().cloned().collect();
        for path in &mut dependencies.paths {
            let mut visited = std::collections::HashSet::new();
            loop {
                let (root, suffix) = path
                    .split_once("::")
                    .map_or((path.as_str(), None), |(root, rest)| (root, Some(rest)));
                let Some(target) = aliases.get(root) else {
                    break;
                };
                if !visited.insert(root.to_owned()) {
                    break;
                }
                *path = suffix.map_or_else(|| target.clone(), |rest| format!("{target}::{rest}"));
            }
        }
        dependencies
    }

    pub fn contains(&self, dependency: &str) -> bool {
        self.paths
            .iter()
            .any(|path| path == dependency || path.starts_with(&format!("{dependency}::")))
    }

    pub fn sqlx_implementation(&self) -> Option<&str> {
        self.paths.iter().find_map(|path| {
            let allowed = ["sqlx::PgPool", "sqlx::Error"]
                .iter()
                .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}::")));
            ((path == "sqlx" || path.starts_with("sqlx::")) && !allowed).then_some(path.as_str())
        })
    }

    pub fn filesystem_probe(&self) -> Option<&str> {
        // `exists` is also a port method; these synchronous filesystem-specific
        // probes have no place in a feature's production implementation.
        ["is_file", "is_dir", "canonicalize", "try_exists"]
            .into_iter()
            .find(|method| {
                self.methods.iter().any(|found| found == method)
                    || self.paths.iter().any(|path| path == &format!(".{method}"))
                    || self.contains(&format!("std::path::Path::{method}"))
            })
    }

    fn import(&mut self, prefix: &str, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                self.import(&format!("{prefix}{}::", path.ident), &path.tree);
            }
            syn::UseTree::Name(name) => {
                let path = if name.ident == "self" {
                    prefix.trim_end_matches("::").to_owned()
                } else {
                    format!("{prefix}{}", name.ident)
                };
                let alias = path.rsplit("::").next().unwrap().to_owned();
                self.aliases.push((alias, path.clone()));
                self.paths.push(path);
            }
            syn::UseTree::Rename(name) => {
                let path = if name.ident == "self" {
                    prefix.trim_end_matches("::").to_owned()
                } else {
                    format!("{prefix}{}", name.ident)
                };
                self.aliases.push((name.rename.to_string(), path.clone()));
                self.paths.push(path);
            }
            syn::UseTree::Glob(_) => {
                self.paths.push(prefix.trim_end_matches("::").into());
                // Resolve only the task API globs this guard owns.
                let names: &[&str] = match prefix {
                    "tokio::task::" => &["spawn", "spawn_local", "spawn_blocking", "JoinSet"],
                    "tokio_util::task::" => &["TaskTracker"],
                    "tokio::" => &["spawn"],
                    _ => &[],
                };
                for name in names {
                    self.aliases
                        .push(((*name).into(), format!("{prefix}{name}")));
                }
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.import(prefix, item);
                }
            }
        }
    }
}

// Evaluate only facts known when test=false. Unknown feature/target cfgs
// stay visible, including any(test, feature=...).
fn production_cfg(meta: &syn::Meta) -> Option<bool> {
    match meta {
        syn::Meta::Path(path) if path.is_ident("test") => Some(false),
        syn::Meta::List(list) => {
            let children = list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .ok()?;
            let values: Vec<_> = children.iter().map(production_cfg).collect();
            if list.path.is_ident("not") && values.len() == 1 {
                return values[0].map(|v| !v);
            }
            if list.path.is_ident("all") {
                if values.contains(&Some(false)) {
                    Some(false)
                } else if values.iter().all(|v| *v == Some(true)) {
                    Some(true)
                } else {
                    None
                }
            } else if list.path.is_ident("any") {
                if values.contains(&Some(true)) {
                    Some(true)
                } else if values.iter().all(|v| *v == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
fn test_only(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || attribute
                .path()
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .eq(["tokio", "test"])
            || (attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Meta>()
                    .is_ok_and(|meta| production_cfg(&meta) == Some(false)))
    })
}

impl<'ast> Visit<'ast> for Dependencies {
    fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
        self.methods.push(expression.method.to_string());
        visit::visit_expr_method_call(self, expression);
    }
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attributes: &[syn::Attribute] = match item {
            syn::Item::Mod(item) => &item.attrs,
            syn::Item::Fn(item) => &item.attrs,
            syn::Item::Use(item) => &item.attrs,
            syn::Item::Impl(item) => &item.attrs,
            syn::Item::Struct(item) => &item.attrs,
            syn::Item::Enum(item) => &item.attrs,
            syn::Item::Const(item) => &item.attrs,
            syn::Item::Static(item) => &item.attrs,
            syn::Item::Type(item) => &item.attrs,
            syn::Item::Macro(item) => &item.attrs,
            syn::Item::Trait(item) => &item.attrs,
            _ => &[],
        };
        if !test_only(attributes) {
            visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        let attributes: &[syn::Attribute] = match item {
            syn::ImplItem::Fn(item) => &item.attrs,
            syn::ImplItem::Const(item) => &item.attrs,
            syn::ImplItem::Type(item) => &item.attrs,
            syn::ImplItem::Macro(item) => &item.attrs,
            _ => &[],
        };
        if !test_only(attributes) {
            visit::visit_impl_item(self, item);
        }
    }
    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        let attributes: &[syn::Attribute] = match item {
            syn::TraitItem::Fn(item) => &item.attrs,
            syn::TraitItem::Const(item) => &item.attrs,
            syn::TraitItem::Type(item) => &item.attrs,
            syn::TraitItem::Macro(item) => &item.attrs,
            _ => &[],
        };
        if !test_only(attributes) {
            visit::visit_trait_item(self, item);
        }
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.import("", &item.tree);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.paths.push(
            path.segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        );
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        // Token groups cover select!/async-stream!/ordinary macro arguments.
        // Literal contents are discarded, so diagnostic text is never code.
        let tokens = item.parse_body::<MacroPaths>().expect("valid token groups");
        self.paths.extend(tokens.0);
        visit::visit_macro(self, item);
    }
}

#[derive(Default)]
struct MacroPaths(Vec<String>);
impl syn::parse::Parse for MacroPaths {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        use syn::ext::IdentExt;
        let mut paths = Vec::new();
        while !input.is_empty() {
            let nested;
            if input.peek(syn::token::Brace) {
                syn::braced!(nested in input);
                paths.extend(nested.parse::<Self>()?.0);
            } else if input.peek(syn::token::Paren) {
                syn::parenthesized!(nested in input);
                paths.extend(nested.parse::<Self>()?.0);
            } else if input.peek(syn::token::Bracket) {
                syn::bracketed!(nested in input);
                paths.extend(nested.parse::<Self>()?.0);
            } else if input.peek(syn::Lit) {
                let _: syn::Lit = input.parse()?;
            } else if input.peek(syn::Token![.]) && input.peek2(syn::Ident::peek_any) {
                let _: syn::Token![.] = input.parse()?;
                paths.push(format!(".{}", input.call(syn::Ident::parse_any)?));
            } else if input.peek(syn::Ident::peek_any) {
                let mut path = input.call(syn::Ident::parse_any)?.to_string();
                while input.peek(syn::Token![::]) && input.peek3(syn::Ident::peek_any) {
                    let _: syn::Token![::] = input.parse()?;
                    path.push_str("::");
                    path.push_str(&input.call(syn::Ident::parse_any)?.to_string());
                }
                paths.push(path);
            } else {
                input.step(|cursor| {
                    cursor
                        .token_tree()
                        .map(|(_, rest)| ((), rest))
                        .ok_or_else(|| cursor.error("expected token"))
                })?;
            }
        }
        Ok(Self(paths))
    }
}

/// Feature tasks are admitted through injected owners. Only CredentialWork
/// owns finite blocking CPU jobs; its semaphore/drain behavior has runtime tests.
pub fn unowned_task(source: &str, credential_owner: bool) -> Option<&'static str> {
    let dependencies = Dependencies::parse(source);
    [
        "tokio::spawn",
        "tokio::task::spawn",
        "tokio_util::task::TaskTracker",
        "tokio::task::JoinSet",
        "tokio::task::spawn_local",
        "tokio::task::spawn_blocking",
    ]
    .into_iter()
    .find(|path| {
        !(credential_owner && *path == "tokio::task::spawn_blocking") && dependencies.contains(path)
    })
}

#[test]
fn task_ownership_predicate_handles_aliases_macros_and_test_boundaries() {
    for source in [
        "fn f() { tokio::spawn(async {}); }",
        "#[custom::test] fn f() { tokio::spawn(async {}); }",
        "use tokio::{task::{spawn as launch}}; fn f() { launch(async {}); }",
        "use tokio::task as jobs; fn f() { jobs::spawn(async {}); }",
        "use tokio::task::{self as jobs}; fn f() { jobs::spawn(async {}); }",
        "use tokio::task::*; fn f() { spawn(async {}); }",
        "use tokio::task as jobs; fn f() { tokio::select! { _ = jobs::spawn(async {}) => {} } }",
        "#[cfg(test)] mod tests {} fn production() { tokio::task::spawn(async {}); }",
        "#[cfg(not(test))] fn f() { tokio::spawn(async {}); }",
        "#[cfg(any(test, feature = \"live\"))] fn f() { tokio::spawn(async {}); }",
        "use tokio::task::JoinSet as Jobs; fn f() { Jobs::new(); }",
        "use tokio_util::task as tasks; fn f() { tasks::TaskTracker::new(); }",
        "fn f() { tokio::task::spawn_blocking(|| {}); }",
    ] {
        assert!(unowned_task(source, false).is_some(), "{source}");
    }
    for source in [
        "#[cfg(test)] mod tests { fn f() { tokio::spawn(async {}); } }",
        "use tokio::task::*; fn f() { tokio::select! { _ = owner.spawn(async {}) => {} } }",
        "#[test] fn f() { tokio::spawn(async {}); }",
        "#[tokio::test] async fn f() { tokio::spawn(async {}); }",
        "#[cfg(all(test, feature = \"fixture\"))] fn f() { tokio::spawn(async {}); }",
        "impl Owner { #[cfg(test)] fn f() { tokio::spawn(async {}); } }",
        "// tokio::spawn(async {});\nfn f() { owner.spawn(async {}); }",
        "fn f() { tracing::info!(\"tokio::spawn(async {})\"); }",
        "fn f() { tokio::select! { _ = read() => { let s = \"tokio::task::spawn\"; } } }",
    ] {
        assert_eq!(unowned_task(source, false), None, "{source}");
    }
    assert_eq!(
        unowned_task("fn f() { tokio::task::spawn_blocking(|| {}); }", true),
        None
    );
    assert!(unowned_task("fn f() { tokio::spawn(async {}); }", true).is_some());
}

#[test]
fn detects_grouped_aliased_and_macro_dependencies_without_skipping_later_production() {
    let dependencies = Dependencies::parse(
        r#"
        #[cfg(test)] use std::fs;
        #[cfg(test)] mod tests { fn fixture() { sqlx::query("test"); } }
        use sqlx::{query as run_query, postgres::PgRow};
        use std::{process::Command as Process, fs::File};
        fn worker() { tokio::select! { _ = sqlx::query_scalar("test") => {} } }
    "#,
    );
    for forbidden in [
        "sqlx::query",
        "sqlx::postgres::PgRow",
        "std::process",
        "std::fs",
        "sqlx::query_scalar",
    ] {
        assert!(dependencies.contains(forbidden), "{forbidden}");
    }
    assert!(!Dependencies::parse("#[cfg(test)] mod tests { use sqlx::Row; }").contains("sqlx"));
    assert!(
        !Dependencies::parse("const TEXT: &str = \"sqlx::query\"; // sqlx::Row").contains("sqlx")
    );
    assert!(
        Dependencies::parse("use sqlx as database;")
            .sqlx_implementation()
            .is_some()
    );
    assert!(
        Dependencies::parse("use sqlx::{PgPool, Error};")
            .sqlx_implementation()
            .is_none()
    );
    assert_eq!(
        Dependencies::parse(
            "fn materialize(path: &Path) { path.join(\".gitmodules\").is_file(); }"
        )
        .filesystem_probe(),
        Some("is_file")
    );
}
