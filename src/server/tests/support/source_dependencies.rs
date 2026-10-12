//! Small source guard: parse imports/paths, including grouped and renamed imports.
//! Inspect macro bodies too, since orchestration often sits inside `select!`.
use syn::visit::{self, Visit};

#[derive(Default)]
pub struct Dependencies {
    paths: Vec<String>,
    macros: Vec<String>,
    methods: Vec<String>,
}

impl Dependencies {
    pub fn parse(source: &str) -> Self {
        let mut dependencies = Self::default();
        dependencies.visit_file(&syn::parse_file(source).expect("valid Rust source"));
        dependencies
    }

    pub fn contains(&self, dependency: &str) -> bool {
        self.paths
            .iter()
            .any(|path| path == dependency || path.starts_with(&format!("{dependency}::")))
            || self.macros.iter().any(|body| body.contains(dependency))
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
                    || self
                        .macros
                        .iter()
                        .any(|body| body.contains(&format!(".{method}(")))
                    || self.contains(&format!("std::path::Path::{method}"))
            })
    }

    fn import(&mut self, prefix: &str, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                self.import(&format!("{prefix}{}::", path.ident), &path.tree);
            }
            syn::UseTree::Name(name) => self.paths.push(format!("{prefix}{}", name.ident)),
            syn::UseTree::Rename(name) => self.paths.push(format!("{prefix}{}", name.ident)),
            syn::UseTree::Glob(_) => self.paths.push(prefix.trim_end_matches("::").into()),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.import(prefix, item);
                }
            }
        }
    }
}

fn test_only(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Path>()
                    .is_ok_and(|path| path.is_ident("test")))
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
            _ => &[],
        };
        if !test_only(attributes) {
            visit::visit_item(self, item);
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
        self.macros.push(
            item.tokens
                .to_string()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect(),
        );
        visit::visit_macro(self, item);
    }
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
