pub fn registry_host(host: &str) -> String {
    let host = host.trim_end_matches('/').to_ascii_lowercase();
    match host.as_str() {
        "hub.docker.com" | "index.docker.io" | "registry-1.docker.io" => "docker.io".into(),
        _ => host,
    }
}
fn explicit_host(reference: &str) -> Option<(&str, &str)> {
    let (host, path) = reference.split_once('/')?;
    (host.contains(['.', ':']) || host.eq_ignore_ascii_case("localhost")).then_some((host, path))
}

pub fn reference_host(reference: &str) -> String {
    explicit_host(reference.trim())
        .map_or_else(|| "docker.io".into(), |(host, _)| registry_host(host))
}

/// Keep the tag case while resolving Docker Hub aliases and official namespaces.
pub fn canonical_reference(reference: &str) -> String {
    with_default_tag(canonical_path(reference.trim()))
}

fn canonical_path(reference: &str) -> String {
    let (host, name) = explicit_host(reference).unwrap_or(("docker.io", reference));
    let host = registry_host(host);
    if host == "docker.io" && !name.contains('/') {
        format!("{host}/library/{name}")
    } else {
        format!("{host}/{name}")
    }
}

pub fn with_default_tag(mut reference: String) -> String {
    if !reference.contains('@')
        && !reference
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .contains(':')
    {
        reference.push_str(":latest");
    }
    reference
}

pub fn qualify_image_reference(host: &str, image: &str) -> Result<String, &'static str> {
    let image = image.trim();
    if image.is_empty() || image.len() > 2048 {
        return Err("The external image reference is invalid.");
    }
    let docker_hub = registry_host(host) == "docker.io";
    let already_qualified = explicit_host(image).is_some();
    let qualified = if docker_hub || already_qualified {
        image.to_owned()
    } else {
        format!(
            "{}/{}",
            host.trim_end_matches('/'),
            image.trim_start_matches('/')
        )
    };
    Ok(with_default_tag(qualified))
}

pub fn pin_to_digest(reference: &str, digest: Option<&str>) -> String {
    let reference = reference.trim();
    let Some(digest) = digest.map(str::trim).filter(|digest| !digest.is_empty()) else {
        return reference.to_owned();
    };
    format!("{}@{digest}", unversioned(reference))
}

fn unversioned(reference: &str) -> &str {
    let name = reference
        .split_once('@')
        .map_or(reference, |(name, _)| name);
    name.rsplit_once(':')
        .filter(|(_, tag)| !tag.contains('/'))
        .map_or(name, |(name, _)| name)
}

pub fn repository(reference: &str) -> Option<String> {
    let value = reference
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase();
    if value.is_empty()
        || value.starts_with("sha256:")
        || value.contains("<none>")
        || value.contains('$')
    {
        return None;
    }
    let path = unversioned(&value);
    if path.split('/').any(str::is_empty) {
        return None;
    }
    Some(canonical_path(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adoption_matches_repositories_independently_of_tags_and_digests() {
        for value in [
            "nginx",
            "docker.io/nginx:Release",
            "index.docker.io/library/nginx@sha256:a",
            "registry-1.docker.io/nginx",
        ] {
            assert_eq!(
                repository(value).as_deref(),
                Some("docker.io/library/nginx")
            );
        }
        assert_eq!(
            repository("team/nginx").as_deref(),
            Some("docker.io/team/nginx")
        );
        assert_eq!(
            repository("registry.test:5000/my/app:Dev@sha256:a").as_deref(),
            Some("registry.test:5000/my/app")
        );
        for value in ["", "sha256:a", "<none>:<none>", "${IMAGE}"] {
            assert_eq!(repository(value), None);
        }
    }
}
