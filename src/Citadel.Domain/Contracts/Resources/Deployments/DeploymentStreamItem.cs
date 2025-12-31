namespace Domain.Contracts.Resources.Deployments;

public sealed record DeploymentStreamItem(
    string? ProgressMessage = null,
    string? ErrorMessage = null,
    DeploymentApplyError? Error = null
    );

public record DeploymentApplyError(long? Code, string? Message);
