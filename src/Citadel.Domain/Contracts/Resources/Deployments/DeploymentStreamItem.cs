using Domain.Contracts.Resources.Images;

namespace Domain.Contracts.Resources.Deployments;

public sealed record DeploymentStreamItem(
    string? Id = null,
    string? Status = null,
    string? Stream = null,
    string? ProgressMessage = null,
    string? ErrorMessage = null,
    ImagePullProgress? Progress = null,
    DeploymentApplyError? Error = null
    );

public record DeploymentApplyError(long? Code, string? Message);
