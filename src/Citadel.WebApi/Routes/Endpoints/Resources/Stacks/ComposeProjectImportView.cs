using Application.Features.Containers.Queries;
using Application.Features.Stacks.Commands;
using Application.Features.Stacks.Queries;
using Domain;
using Domain.Entities.Stacks;
using WebApi.Routes.Endpoints.Resources.Containers;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record ComposeProjectImportSourceView(
    Guid PlatformId,
    string PlatformName,
    string ProjectName,
    IReadOnlyList<string> ContainerIds,
    IReadOnlyList<string> ContainerNames,
    IReadOnlyList<ComposeProjectRuntimeService> Services);

public sealed record ComposeProjectStackDraftView(
    string Name,
    Guid PlatformId,
    string? Description,
    StackDriftPolicy DriftPolicy,
    IReadOnlyCollection<Guid> TagIds);

public sealed record ComposeProjectImportDraftView(
    StackImportKind ImportKind,
    ComposeProjectImportSourceView Source,
    ComposeProjectStackDraftView Draft,
    IReadOnlyCollection<ContainerAdoptionIssueView> Issues,
    string RuntimeFingerprint)
{
    internal static ComposeProjectImportDraftView Map(ComposeProjectImportDraft draft)
        => new(
            draft.ImportKind,
            new ComposeProjectImportSourceView(
                draft.Source.PlatformId,
                draft.Source.PlatformName,
                draft.Source.ProjectName,
                draft.Source.ContainerIds,
                draft.Source.ContainerNames,
                draft.Source.Services),
            new ComposeProjectStackDraftView(
                draft.Draft.Name,
                draft.Draft.PlatformId,
                draft.Draft.Description,
                draft.Draft.DriftPolicy,
                draft.Draft.TagIds),
            [.. draft.Issues.Select(issue => new ContainerAdoptionIssueView(
                issue.Code,
                issue.Message,
                issue.Severity,
                issue.FieldPath))],
            draft.RuntimeFingerprint);
}

public sealed record ValidateComposeProjectImportInput(
    string Name,
    StackSource StackSource,
    StackSpec Spec,
    StackImportKind? ImportKind = null)
{
    internal ValidateComposeProjectImportDraft ToQuery(Guid platformId, string projectName)
        => new(platformId, projectName, Name, StackSource, Spec, ImportKind);
}

public sealed record ImportComposeProjectInput(
    string Name,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null,
    StackImportKind? ImportKind = null,
    bool ImportSensitiveEnvironmentAsSecrets = false)
{
    internal ImportComposeProject ToCommand(Guid platformId, string projectName)
        => new(
            platformId,
            projectName,
            Name,
            Description,
            StackSource,
            Spec,
            PreviewFingerprint,
            TagIds,
            ImportKind,
            ImportSensitiveEnvironmentAsSecrets);
}
