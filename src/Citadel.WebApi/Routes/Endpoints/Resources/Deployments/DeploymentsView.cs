
using Domain;
using Domain.Entities;
namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentsView(IEnumerable<DeploymentView> Deployments);

public sealed record DeploymentView(
    Guid Id,
    string Name,
    string? Description,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    Guid CreatedBy,
    Guid? UpdatedBy,
    DeploymentVersionView? ActiveVersion
    );

public sealed record DeploymentVersionView(
    Guid Id,
    Guid DeploymentId,
    int Version,
    Guid PlatformId,
    DeploymentSpec Spec,
    DeploymentStatus Status,
    DeploymentSource Source,
    Guid CreatedBy,
    DateTime CreatedAt,
    DateTime? UpdatedAt,
    Guid? UpdatedBy,
    int? RolledBackFromVersion,
    string? GitRepoUrl,
    string? GitPath,
    string? GitCommitHash
    );