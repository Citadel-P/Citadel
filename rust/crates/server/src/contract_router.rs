use axum::Router;
use axum::handler::Handler;
use axum::routing::{MethodFilter, on};
use citadel_contracts::http::{HttpMethod, RouteContract};

pub trait ContractRouterExt<S> {
    fn contract_route<H, T>(self, contract: RouteContract, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static;
}

impl<S> ContractRouterExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn contract_route<H, T>(self, contract: RouteContract, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let filter = match contract.method {
            HttpMethod::Get => MethodFilter::GET,
            HttpMethod::Post => MethodFilter::POST,
            HttpMethod::Put => MethodFilter::PUT,
            HttpMethod::Patch => MethodFilter::PATCH,
            HttpMethod::Delete => MethodFilter::DELETE,
        };
        self.route(&runtime_path(contract.path), on(filter, handler))
    }
}

fn runtime_path(contract_path: &str) -> String {
    let mut position = 0_usize;
    contract_path
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                let catch_all = segment.starts_with("{*");
                let name = if catch_all {
                    format!("{{*parameter{position}}}")
                } else {
                    format!("{{parameter{position}}}")
                };
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use citadel_contracts::http::routes;
    use tower::ServiceExt;

    #[tokio::test]
    async fn contract_controls_registered_path_and_method() {
        let app = Router::new().contract_route(routes::GET_HEALTH, || async { "ok" });

        let get = app
            .clone()
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let post = app
            .oneshot(Request::post("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(get.status(), StatusCode::OK);
        assert_eq!(post.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn equivalent_contract_paths_can_use_different_openapi_parameter_names() {
        let app = Router::new()
            .contract_route(routes::GET_DEPLOYMENT, || async { StatusCode::OK })
            .contract_route(routes::UPDATE_DEPLOYMENT, || async {
                StatusCode::NO_CONTENT
            });

        assert_eq!(
            app.clone()
                .oneshot(
                    Request::get("/api/v1/deployments/00000000-0000-0000-0000-000000000001")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(
                Request::patch("/api/v1/deployments/00000000-0000-0000-0000-000000000001",)
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
            StatusCode::NO_CONTENT
        );
    }
}
