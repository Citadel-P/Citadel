using Application.Features.Containers.Commands;
using Application.Features.Containers.Queries;
using Domain;
using Domain.Entities.Deployments;
using WebApi.Routes.Endpoints.Resources.Deployments;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record ContainerAdoptionSourceView(
    Guid Id,
    string DockerContainerId,
    string Name,
    Guid PlatformId,
    string PlatformName,
    ContainerStateStatus State);

public sealed record ContainerAdoptionIssueView(
    string Code,
    string Message,
    AdoptionIssueSeverity Severity,
    string? FieldPath);

public sealed record ContainerAdoptionDraftView(
    ContainerAdoptionSourceView Source,
    CreateDeploymentInput Draft,
    IReadOnlyCollection<ContainerAdoptionIssueView> Issues,
    string PreviewFingerprint,
    bool CanImportSensitiveEnvironmentValues)
{
    internal static ContainerAdoptionDraftView Map(ContainerAdoptionDraft draft)
        => new(
            new ContainerAdoptionSourceView(
                draft.Source.Id,
                draft.Source.DockerContainerId,
                draft.Source.Name,
                draft.Source.PlatformId,
                draft.Source.PlatformName,
                draft.Source.State),
            new CreateDeploymentInput(
                draft.Draft.Name,
                draft.Draft.PlatformId,
                draft.Draft.Description,
                draft.Draft.Spec,
                draft.Draft.TagIds),
            [.. draft.Issues.Select(issue => new ContainerAdoptionIssueView(
                issue.Code,
                issue.Message,
                issue.Severity,
                issue.FieldPath))],
            draft.PreviewFingerprint,
            draft.CanImportSensitiveEnvironmentValues);
}

public sealed record AdoptContainerInput(
    string Name,
    string? Description,
    DeploymentSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null,
    bool ImportSensitiveEnvironmentAsSecrets = false)
{
    internal AdoptContainer ToCommand(Guid containerId)
        => new(
            containerId,
            Name,
            Description,
            Spec,
            PreviewFingerprint,
            TagIds,
            ImportSensitiveEnvironmentAsSecrets);
}
