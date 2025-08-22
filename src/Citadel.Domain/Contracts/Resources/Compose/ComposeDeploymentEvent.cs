using Domain.Contracts.Resources.Images;

namespace Domain.Contracts.Resources.Compose;

public abstract record ComposeDeploymentEvent(string StepId, DateTime? Timestamp = null);
public sealed record StepStartedEvent(string StepId, string Description) : ComposeDeploymentEvent(StepId);
public sealed record StepProgressEvent(string StepId, string Message, double? Percent = null) : ComposeDeploymentEvent(StepId);
public sealed record StepCompletedEvent(string StepId, string? Message = null, string? ResourceId = null) : ComposeDeploymentEvent(StepId);
public sealed record StepFailedEvent(string StepId, string Error) : ComposeDeploymentEvent(StepId);
public sealed record PullProgressEvent(string StepId, string Image, string? Id, string? Status, string? ProgressMessage, ImagePullProgress? PullProgress) : ComposeDeploymentEvent(StepId);
