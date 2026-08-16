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
            HttpMethod::Patch => MethodFilter::PATCH,
            HttpMethod::Delete => MethodFilter::DELETE,
        };
        self.route(contract.path, on(filter, handler))
    }
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
}
