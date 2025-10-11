
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentView(
    Guid Id,
    string Name,
    string? Description,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    Guid CreatedBy,
    int ActiveVersion,
    IEnumerable<DeploymentVersion> Versions
    );
