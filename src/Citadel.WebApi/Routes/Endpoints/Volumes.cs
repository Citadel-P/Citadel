using Application.Features.Backups.Models;
using Application.Features.Backups.Queries;
using Application.Features.Volumes.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Volumes;
using Hosting.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Volumes;

namespace WebApi.Routes.Endpoints;

public static class Volumes
{
    public static async Task<Results<Ok<VolumesView>, ProblemHttpResult>> List(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [Description("The platform id")] Guid platformId,
        [AsParameters] ListVolumesRequest listNetworksRequest,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(platformId), cancellationToken);
        if (!result.IsSuccess(out var volumes))
            return await EndpointHandlers.HandleResult(result, permissionEvaluator, VolumesView.Map);

        var list = volumes as DockerVolumeResult[] ?? [.. volumes];
        var coverage = await GetBackupCoverageAsync(mediator, list, cancellationToken);
        return TypedResults.Ok(await VolumesView.Map(list, permissionEvaluator, coverage));
    }

    public static async Task<Results<Ok<DockerVolumeResultView>, ProblemHttpResult>> Create(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        [FromBody] CreateVolumeInput request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerVolumeResultView.Map);
    }

    public static async Task<Results<Ok<DockerVolumeResultView>, ProblemHttpResult>> Inspect(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string name,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectVolume(platformId, name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerVolumeResultView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(
        IMediator mediator,
        [FromBody] DeleteVolumesInput request,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }

    private static async Task<IReadOnlyDictionary<VolumeBackupCoverageKey, BackupCoverageView>?> GetBackupCoverageAsync(
        IMediator mediator,
        IReadOnlyCollection<DockerVolumeResult> volumes,
        CancellationToken cancellationToken)
    {
        if (volumes.Count == 0)
            return null;

        var resources = volumes
            .Where(static volume => volume.PlatformId != Guid.Empty && !string.IsNullOrWhiteSpace(volume.Name))
            .Select(static volume => new BackupCoverageResourceKey(PlatformId: volume.PlatformId, Name: volume.Name))
            .ToArray();

        if (resources.Length == 0)
            return null;

        var result = await mediator.Send(new GetBackupCoverage(BackupCoverageResourceType.Volume, resources), cancellationToken);
        if (!result.IsSuccess(out var coverage))
            return null;

        return coverage.Items
            .Where(static item => item.Resource.PlatformId.HasValue && !string.IsNullOrWhiteSpace(item.Resource.Name))
            .ToDictionary(
                static item => new VolumeBackupCoverageKey(item.Resource.PlatformId!.Value, item.Resource.Name!),
                static item => item.Coverage);
    }
}
