using Application.Features.SwarmServices.Commands;
using Application.Features.SwarmServices.Queries;
using Domain.Entities.SwarmServices;

namespace WebApi.Routes.Endpoints.Resources.SwarmServices;

public sealed record SwarmServiceAdoptionSourceView(
    string DockerServiceId,
    string Name,
    Guid PlatformId,
    string PlatformName);

public sealed record SwarmServiceAdoptionIssueView(string Code, string Message);

public sealed record SwarmServiceAdoptionDraftView(
    SwarmServiceAdoptionSourceView Source,
    CreateSwarmServiceInput Draft,
    IReadOnlyList<SwarmServiceAdoptionIssueView> Issues,
    string PreviewFingerprint)
{
    internal static SwarmServiceAdoptionDraftView Map(SwarmServiceAdoptionDraft value) => new(
        new SwarmServiceAdoptionSourceView(
            value.Source.DockerServiceId,
            value.Source.Name,
            value.Source.PlatformId,
            value.Source.PlatformName),
        new CreateSwarmServiceInput(
            value.Name,
            value.Source.PlatformId,
            value.Description,
            value.Spec),
        value.Issues.Select(static issue => new SwarmServiceAdoptionIssueView(issue.Code, issue.Message)).ToArray(),
        value.PreviewFingerprint);
}

public sealed record AdoptSwarmServiceInput(
    string Name,
    string? Description,
    SwarmServiceSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal AdoptSwarmService ToCommand(Guid platformId, string dockerServiceId) => new(
        platformId,
        dockerServiceId,
        Name,
        Description,
        Spec,
        PreviewFingerprint,
        TagIds);
}
