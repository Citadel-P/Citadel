
namespace WebApi.Routes.Endpoints.Resources.Deployments;

public sealed record DeploymentsView(IEnumerable<DeploymentInfoView> Deployments);

public sealed record DeploymentInfoView(
    Guid Id,
    string Name,
    string? Description,
    DateTime CreatedAt,
    DateTime UpdatedAt,
    Guid CreatedBy,
    int ActiveVersion,
    int VersionCount
    );
