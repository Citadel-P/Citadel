//! Named request examples shared by registry create and PATCH documentation.
use serde_json::{Value, json};

pub(crate) fn azure() -> Value {
    json!({"name": "my-azure-registry", "registryHost": "myproject.azurecr.io", "status": "Active", "configuration": {"$type": "Azure", "userName": "example-user", "password": "example-password"}})
}

pub(crate) fn aws() -> Value {
    json!({"name": "my-ecr-registry", "registryHost": "123456789012.dkr.ecr.us-east-2.amazonaws.com", "status": "Active", "configuration": {"$type": "AWS", "accessKey": "example-access-key", "secretAccessKey": "example-secret-key", "region": "us-east-2", "authenticationRequired": true}})
}

pub(crate) fn gitlab() -> Value {
    json!({"name": "my-gitlab-registry", "registryHost": "registry.gitlab.com", "status": "Active", "configuration": {"$type": "Gitlab", "userName": "example-user", "pat": "example-token", "instanceUrl": "https://gitlab.com"}})
}

pub(crate) fn dockerhub() -> Value {
    json!({"name": "my-dockerhub-registry", "registryHost": "docker.io", "status": "Active", "configuration": {"$type": "DockerHub", "userName": "example-user", "pat": "example-token"}})
}

pub(crate) fn github() -> Value {
    json!({"name": "my-github-registry", "registryHost": "ghcr.io", "status": "Active", "configuration": {"$type": "GitHub", "nameSpace": "citadel-p", "ghcrAuthEnabled": true, "pat": "example-token"}})
}

pub(crate) fn custom() -> Value {
    json!({"name": "my-custom-registry", "registryHost": "localhost:9965", "status": "Active", "configuration": {"$type": "Custom", "authEnabled": true, "userName": "example-user", "password": "example-password"}})
}
