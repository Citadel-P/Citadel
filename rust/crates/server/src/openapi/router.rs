//! Preserve documented API parameter names when registering Axum paths.
//!
//! Axum requires `/resources/{id}` and `/resources/{resourceId}/config` to use
//! the same capture name. Extractors in Citadel read captures positionally.
use utoipa_axum::router::{OpenApiRouter, UtoipaMethodRouter};

pub trait OpenApiRouterExt<S> {
    fn normalized_routes(self, routes: UtoipaMethodRouter<S>) -> Self;
}

impl<S: Clone + Send + Sync + 'static> OpenApiRouterExt<S> for OpenApiRouter<S> {
    fn normalized_routes(self, (schemas, paths, handler): UtoipaMethodRouter<S>) -> Self {
        let (mut router, mut api) = self.split_for_parts();
        for (path, item) in paths.paths {
            router = router.route(&runtime_path(&path), handler.clone());
            if let Some(existing) = api.paths.paths.get_mut(&path) {
                existing.merge_operations(item);
            } else {
                api.paths.paths.insert(path, item);
            }
        }
        api.components
            .get_or_insert_with(Default::default)
            .schemas
            .extend(schemas);
        OpenApiRouter::with_openapi(api).merge(router.into())
    }
}

fn runtime_path(path: &str) -> String {
    let mut position = 0;
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                let prefix = if segment.starts_with("{*") { "*" } else { "" };
                let name = format!("{{{prefix}parameter{position}}}");
                position += 1;
                name
            } else {
                segment.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[utoipa::path(get, path = "/resources/{id}", params(("id" = String, Path)), responses((status = 200)))]
    async fn read() -> StatusCode {
        StatusCode::OK
    }

    #[utoipa::path(patch, path = "/resources/{resourceId}", params(("resourceId" = String, Path)), responses((status = 204)))]
    async fn update() -> StatusCode {
        StatusCode::NO_CONTENT
    }

    #[tokio::test]
    async fn routing_uses_documented_methods_and_preserves_parameter_names() {
        let (router, api) = OpenApiRouter::new()
            .normalized_routes(utoipa_axum::routes!(read))
            .normalized_routes(utoipa_axum::routes!(update))
            .split_for_parts();
        assert!(api.paths.paths["/resources/{id}"].get.is_some());
        assert!(api.paths.paths["/resources/{resourceId}"].patch.is_some());
        for (method, expected) in [
            ("GET", StatusCode::OK),
            ("PATCH", StatusCode::NO_CONTENT),
            ("POST", StatusCode::METHOD_NOT_ALLOWED),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/resources/example")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }
}
