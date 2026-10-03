use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ResponseContent<T> {
    pub status: reqwest::StatusCode,
    pub content: String,
    pub entity: Option<T>,
}

#[derive(Debug)]
pub enum Error<T> {
    ResponseTooLarge {
        limit: usize,
        status: reqwest::StatusCode,
    },
    ResponseRead {
        status: reqwest::StatusCode,
        source: reqwest::Error,
    },
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ResponseContent<T>),
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (module, e) = match self {
            Error::ResponseTooLarge { limit, .. } => {
                ("response", format!("body exceeds {limit} bytes"))
            }
            Error::ResponseRead { source, .. } => ("response", source.to_string()),
            Error::Reqwest(e) => ("reqwest", e.to_string()),
            Error::Serde(e) => ("serde", e.to_string()),
            Error::Io(e) => ("IO", e.to_string()),
            Error::ResponseError(e) => ("response", format!("status code {}", e.status)),
        };
        write!(f, "error in {}: {}", module, e)
    }
}

impl<T: fmt::Debug> error::Error for Error<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(match self {
            Error::ResponseTooLarge { .. } => return None,
            Error::ResponseRead { source, .. } => source,
            Error::Reqwest(e) => e,
            Error::Serde(e) => e,
            Error::Io(e) => e,
            Error::ResponseError(_) => return None,
        })
    }
}

impl<T> From<reqwest::Error> for Error<T> {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl<T> From<serde_json::Error> for Error<T> {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

impl<T> From<std::io::Error> for Error<T> {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub fn urlencode<T: AsRef<str>>(s: T) -> String {
    ::url::form_urlencoded::byte_serialize(s.as_ref().as_bytes()).collect()
}

pub fn parse_deep_object(prefix: &str, value: &serde_json::Value) -> Vec<(String, String)> {
    if let serde_json::Value::Object(object) = value {
        let mut params = vec![];

        for (key, value) in object {
            match value {
                serde_json::Value::Object(_) => params.append(&mut parse_deep_object(
                    &format!("{}[{}]", prefix, key),
                    value,
                )),
                serde_json::Value::Array(array) => {
                    for (i, value) in array.iter().enumerate() {
                        params.append(&mut parse_deep_object(
                            &format!("{}[{}][{}]", prefix, key, i),
                            value,
                        ));
                    }
                }
                serde_json::Value::String(s) => {
                    params.push((format!("{}[{}]", prefix, key), s.clone()))
                }
                _ => params.push((format!("{}[{}]", prefix, key), value.to_string())),
            }
        }

        return params;
    }

    unimplemented!("Only objects are supported with style=deepObject")
}

/// Internal use only
/// A content type supported by this client.
#[allow(dead_code)]
enum ContentType {
    Json,
    Text,
    Unsupported(String),
}

impl From<&str> for ContentType {
    fn from(content_type: &str) -> Self {
        if content_type.starts_with("application") && content_type.contains("json") {
            return Self::Json;
        } else if content_type.starts_with("text/plain") {
            return Self::Text;
        } else {
            return Self::Unsupported(content_type.to_string());
        }
    }
}

pub mod config_api;
pub mod container_api;
pub mod distribution_api;
pub mod exec_api;
pub mod image_api;
pub mod network_api;
pub mod node_api;
pub mod plugin_api;
pub mod secret_api;
pub mod service_api;
pub mod session_api;
pub mod swarm_api;
pub mod system_api;
pub mod task_api;
pub mod volume_api;

pub mod configuration;

/// Read finite responses without trusting Content-Length or buffering an unbounded body.
async fn bounded_text<T>(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, Error<T>> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(Error::ResponseTooLarge {
            limit,
            status: response.status(),
        });
    }
    let mut body = Vec::new();
    let status = response.status();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|source| Error::ResponseRead { status, source })?
    {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(Error::ResponseTooLarge {
                limit,
                status: response.status(),
            });
        }
        body.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Ok(String::from_utf8_lossy(&body).into_owned());
    }
    String::from_utf8(body).map_err(|error| {
        Error::Serde(serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error,
        )))
    })
}
