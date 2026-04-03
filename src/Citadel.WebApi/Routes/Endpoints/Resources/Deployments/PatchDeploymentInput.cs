using Domain.Entities.Deployments;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record PatchDeploymentInput(Guid PlatformId, DeploymentSpec Spec);