---
title: "Registries"
description: "Configure image registries used by Deployments, Stacks, Services, and Builds."
---

Registries let Citadel pull container images, check image updates, and push images created by builds.

Use registries when:

- a deployment pulls an external image
- a web editor stack checks service images for updates
- a build pushes a generated image
- you want to pull an image onto a platform from Citadel
- private registry credentials are required

## Use public or private images

For a public Docker Hub image, select the built-in **Docker Hub** registry in
the application form. You do not need to add credentials.

For a private image:

1. Open **Registries** and select **Add Registry**.
2. Choose **DockerHub**, **GitHub**, or **Custom**, then enter the required provider settings.
3. Select **Save**. Citadel checks supported credentials before saving.
4. Choose this Registry in your Deployment, Stack, or Build Project.

Use a provider token with the access needed for your task. Pulling and pushing
images may require different permissions.

## Supported Providers

The registry form supports:

- `DockerHub`
- `GitHub`
- `Custom`

DockerHub uses `docker.io` as the registry host. GitHub uses `ghcr.io` and displays the selected namespace as part of the host. Custom registries require you to enter the registry host.

Azure, AWS, and GitLab registry configuration types exist in the backend model, but they are not currently exposed in the registry form.

Builds currently use DockerHub, GitHub Container Registry, and Custom registries for pushes.

## Basic Setup

Create a registry and choose:

- Provider: DockerHub, GitHub, or Custom
- Name: a unique name used inside Citadel
- Description: optional notes
- Tags: optional filters for organizing registries
- Status: whether the registry is active, deprecated, or disabled
- Provider settings: credentials and provider-specific fields

Citadel validates supported registry credentials when the registry is created or updated.

## Registry Status

Registry status controls how the registry is presented for use.

- `Active`: available for normal selection and use.
- `Deprecated`: still available, but shown with a warning.
- `Disabled`: hidden from selection and cannot be used for new configuration.

Use `Deprecated` when you are migrating workloads away from a registry but existing resources still depend on it. Use `Disabled` when the registry should no longer be selected.

## Default Registry

Citadel includes a default registry entry. The default registry cannot be edited or deleted.

Use the default registry for public images that do not require credentials. Create a separate registry when you need private credentials, provider-specific package discovery, or clearer ownership in deployment and stack configuration.

## DockerHub

Use DockerHub for images hosted on Docker Hub.

Required fields:

- Username
- Personal access token

The DockerHub registry host is set to `docker.io` automatically.

Use a DockerHub personal access token instead of an account password. The token is used when Citadel pulls images, checks tags, and lists DockerHub repositories or tags where supported.

Example image references:

```text
nginx:1.27
library/postgres:16
my-org/my-app:1.4.2
```

## GitHub Container Registry

Use GitHub for images hosted in GitHub Container Registry.

Required fields:

- Namespace: the GitHub user or organization that owns the packages
- Authentication: optional for public packages, required for private packages
- PAT: required when authentication is enabled

The GitHub registry host is set to `ghcr.io` automatically. In the registry list, Citadel displays the host with the namespace, for example:

```text
ghcr.io/my-org
```

For private packages, enable authentication and provide a GitHub personal access token with package read access.

Example image references:

```text
ghcr.io/my-org/api:1.4.2
ghcr.io/my-user/worker:latest
```

For public packages, authentication is optional, but it can help avoid GitHub rate limits.

## Custom Registry

Use Custom for an OCI-compatible registry that is not DockerHub or GitHub Container Registry.

Required fields:

- Registry Host: host or `host:port`, without `http://` or `https://`
- Authentication: enable only when credentials are required
- Username and password: required when authentication is enabled

Examples:

```text
registry.example.com
registry.example.com:5000
10.0.10.15:5000
```

Example image references:

```text
registry.example.com/team/api:1.4.2
registry.example.com:5000/internal/job:2026.07
```

Citadel assumes custom registry connectivity is valid when saving the registry. If credentials, TLS, or network access are wrong, image pull or update checks can fail later during deployment.

## Using Registries In Deployments

For a deployment with image source `External`, select a registry and enter an image reference.

During deploy, Citadel:

1. pulls the image from the selected registry
2. resolves the local Docker image id
3. injects deployment variables and secrets
4. creates or recreates the container

Auto update for deployments also depends on the selected registry. It checks the configured external image tag for a new digest.

For deployment setup, see [Deployments](/docs/resources/deployments).

## Using Registries In Web Editor Stacks

Select a Registry when creating a Web Editor Stack. Citadel uses its credentials
for deployment and service-image update checks on Docker Standalone and Swarm.
The selected Platform determines whether deployment uses Docker Compose or a
native Swarm Stack. The Compose definition or Build Images bindings determine
the images used by its services.

For web editor stack setup, see [Web Editor Stacks](/docs/resources/stacks/web-editor).

## Using Registries In Builds

Builds use registries as image push targets.

In a build project, select:

- Registry: the registry that owns the image repository.
- Image repository: repository path under that registry, without the host or tag.
- Tags: one or more tag templates.

Example with GitHub Container Registry:

```text
Registry: ghcr.io/acme
Image repository: platform/api
Tags: {branch}-{shortSha}, latest
```

Citadel resolves the final image references during the build run:

```text
ghcr.io/acme/platform/api:main-a4c8e3c1d420
ghcr.io/acme/platform/api:latest
```

Private image pulls need read access. Publishing Build output also needs push
access to the destination repository.

For build setup, see [Builds](/docs/resources/builds).

## Pulling Images To A Platform

Registries are also used when pulling an image into a platform from the Images page.

Select:

- the target platform
- the registry
- the image reference

Citadel uses the registry credentials to pull the image onto the selected Docker host. After that, the image can be selected as a local deployment image.

## Credentials And Secrets

Registry credentials are stored in the registry configuration. Sensitive values are masked in snapshots and activity details.

Use least-privilege tokens:

- DockerHub: token that can read or write the required repositories, depending on use
- GitHub Container Registry: token with package read or write access, depending on use
- Custom registry: read-only credentials for pull-only use, or write credentials for builds

To rotate a token, issue a replacement at the provider, update the Registry in
Citadel, and save. Verify a pull on an intended Platform, or a Build push if the
Registry is a build destination, before revoking the old token. Updating
credentials does not recreate running containers.

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Unauthorized or denied | Token validity, repository/package access, and whether the operation needs pull or push permission. Git source credentials and Registry credentials are separate. |
| Image or manifest not found | Registry host, namespace, repository, and exact tag or digest. A private registry may also hide an unauthorized repository this way. |
| Save succeeds but a pull or push fails | Connectivity and certificate trust on the executing Docker host or builder. Core credential validation does not verify every execution host. |
| Registry missing from a selector | Registry status, provider support for that operation, and your access. |
| No image update is detected | The deployed source must be a supported tag with a known applied digest; digest pins and Build artifacts use different update paths. |

Inspect the failed operation's logs before retrying. Confirm the resulting image
reference and application health after a successful deployment; see
[Update applications](/docs/guides/application-updates).

## Choosing A Registry

Use DockerHub when:

- images are hosted on Docker Hub
- DockerHub repository/tag browsing is useful
- DockerHub credentials are required for private images

Use GitHub when:

- images are hosted on `ghcr.io`
- images belong to a GitHub user or organization namespace
- private GHCR packages need package read credentials

Use Custom when:

- images are hosted on a private OCI-compatible registry
- the registry uses a custom host or port
- the registry does not match DockerHub or GHCR behavior
