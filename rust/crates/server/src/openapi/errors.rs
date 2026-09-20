//! Reusable Problem Details responses shared by handler annotations.
use crate::api::error::ProblemDetails;

macro_rules! problem_responses {
    ($name:ident { $($variant:ident => ($status:literal, $description:literal)),* $(,)? }) => {
        #[allow(dead_code)]
        #[derive(utoipa::IntoResponses)]
        pub enum $name {
            $(#[response(status = $status, description = $description, content_type = "application/problem+json")]
            $variant(ProblemDetails),)*
            #[response(status = "default", description = "Request failed", content_type = "application/problem+json")]
            Unexpected(ProblemDetails),
        }
    };
}

problem_responses! { RequestErrors {
    BadRequest => (400, "Bad Request"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { ReadinessErrors {
    BadRequest => (400, "Bad Request"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    ServiceUnavailable => (503, "Service Unavailable"),
} }

problem_responses! { InitializationErrors {
    BadRequest => (400, "Bad Request"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    ServiceUnavailable => (503, "Service Unavailable"),
} }

problem_responses! { AuthenticationErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { LoginErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    NotFound => (404, "Not Found"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { RedirectErrors {
    BadRequest => (400, "Bad Request"),
    NotFound => (404, "Not Found"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { AccessErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { ExternalAccessErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    BadGateway => (502, "Bad Gateway"),
} }

problem_responses! { CreateErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { ResourceErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    NotFound => (404, "Not Found"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { ResourceMutationErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    NotFound => (404, "Not Found"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
} }

problem_responses! { ExternalResourceErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    NotFound => (404, "Not Found"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    BadGateway => (502, "Bad Gateway"),
} }

problem_responses! { UnavailableResourceErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    NotFound => (404, "Not Found"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    ServiceUnavailable => (503, "Service Unavailable"),
} }

problem_responses! { ExternalRuntimeErrors {
    BadRequest => (400, "Bad Request"),
    Unauthorized => (401, "Unauthorized"),
    Forbidden => (403, "Forbidden"),
    NotFound => (404, "Not Found"),
    Conflict => (409, "Conflict"),
    TooManyRequests => (429, "Too Many Requests"),
    InternalServerError => (500, "Internal Server Error"),
    BadGateway => (502, "Bad Gateway"),
    ServiceUnavailable => (503, "Service Unavailable"),
} }
