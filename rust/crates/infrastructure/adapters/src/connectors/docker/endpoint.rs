//! One daemon endpoint for generated HTTP operations, streams and Docker CLI work.
use std::{path::PathBuf, str::FromStr};
use url::Url;

use super::DockerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerEndpoint {
    Unix(PathBuf),
    Tcp(Url),
}

impl FromStr for DockerEndpoint {
    type Err = DockerError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let endpoint = Url::parse(value.trim()).map_err(|_| {
            DockerError::InvalidEndpoint("expected an absolute unix, tcp or http URL")
        })?;
        let parsed = match endpoint.scheme() {
            "unix"
                if endpoint.host_str().is_none()
                    && endpoint.query().is_none()
                    && endpoint.fragment().is_none() =>
            {
                if !endpoint.path().starts_with('/') || endpoint.path().len() < 2 {
                    return Err(DockerError::InvalidEndpoint(
                        "expected an absolute socket path",
                    ));
                }
                let file = Url::parse(&format!("file://{}", endpoint.path()))
                    .ok()
                    .and_then(|url| url.to_file_path().ok())
                    .ok_or(DockerError::InvalidEndpoint("invalid socket path"))?;
                Self::Unix(file)
            }
            "tcp" | "http" => Self::Tcp(endpoint),
            _ => {
                return Err(DockerError::InvalidEndpoint(
                    "supported schemes are unix, tcp and http",
                ));
            }
        };
        parsed.validate()?;
        Ok(parsed)
    }
}

impl DockerEndpoint {
    pub(super) fn validate(&self) -> Result<(), DockerError> {
        let valid = match self {
            Self::Unix(path) => {
                path.is_absolute()
                    && path.parent().is_some()
                    && !path.as_os_str().as_encoded_bytes().contains(&0)
            }
            Self::Tcp(url) => {
                matches!(url.scheme(), "tcp" | "http")
                    && url.host_str().is_some()
                    && url.port_or_known_default().is_some_and(|port| port > 0)
                    && url.username().is_empty()
                    && url.password().is_none()
                    && matches!(url.path(), "" | "/")
                    && url.query().is_none()
                    && url.fragment().is_none()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(DockerError::InvalidEndpoint(
                "invalid daemon address or socket path",
            ))
        }
    }

    pub(super) fn http_origin(&self) -> String {
        match self {
            Self::Unix(_) => "http://localhost".into(),
            Self::Tcp(url) => format!("http://{}", url.authority()),
        }
    }

    pub fn docker_host(&self) -> Result<String, DockerError> {
        self.validate()?;
        match self {
            Self::Unix(path) => Url::from_file_path(path)
                .map(|url| format!("unix://{}", url.path()))
                .map_err(|_| DockerError::InvalidEndpoint("invalid socket path")),
            Self::Tcp(url) => Ok(self.http_origin().replacen("http://", "tcp://", 1)
                + if url.port().is_none() { ":80" } else { "" }),
        }
    }
}
