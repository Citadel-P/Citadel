using Application.Features.Backups.Models;
using Application.Features.Backups.Queries;
using Application.Features.Volumes.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Volumes;
using Domain.Entities.Activities;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Net.Http.Headers;
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
        [FromQuery] string? dockerNodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectVolume(platformId, name, dockerNodeId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerVolumeResultView.Map);
    }

    public static async Task<Results<Ok<VolumeDirectoryView>, ProblemHttpResult>> ListDirectory(
        IMediator mediator,
        Guid platformId,
        string name,
        [FromQuery] string? path,
        [FromQuery] string? dockerNodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new ListVolumeDirectory(platformId, name, path, dockerNodeId),
            cancellationToken);
        return EndpointHandlers.HandleResult(result, VolumeDirectoryView.Map);
    }

    public static async Task<IResult> Download(
        IMediator mediator,
        HttpContext httpContext,
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContextAccessor,
        Guid platformId,
        string name,
        [FromQuery] string? path,
        [FromQuery] string? dockerNodeId,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(
            new OpenVolumeDownload(platformId, name, path, dockerNodeId),
            cancellationToken);
        if (!result.IsSuccess(out var download))
            return EndpointHandlers.HandleResult(result, static _ => new object());

        await using (download)
        {
            var response = httpContext.Response;
            response.ContentType = download.ContentType;
            response.ContentLength = download.ContentLength;
            response.Headers.CacheControl = "no-store";
            response.Headers.Pragma = "no-cache";
            response.Headers.XContentTypeOptions = "nosniff";

            var disposition = new ContentDispositionHeaderValue("attachment");
            disposition.SetHttpFileName(download.FileName);
            response.Headers.ContentDisposition = disposition.ToString();

            await foreach (var chunk in download.Chunks.WithCancellation(cancellationToken))
            {
                await response.Body.WriteAsync(chunk, cancellationToken);
            }

            var actorId = userContextAccessor.Current.ActorId == Guid.Empty
                ? Constants.SystemId
                : userContextAccessor.Current.ActorId;

            await unitOfWork.ActivityEventRepository.AddAsync(
                new ActivityEvent(
                    platformId: platformId,
                    resourceId: platformId,
                    actorId: actorId,
                    resourceName: name,
                    eventType: ActivityEventType.VolumeContentDownloaded,
                    status: ActivityStatus.Success,
                    info: new VolumeContentDownloaded(
                        VolumeName: name,
                        Path: download.Path,
                        IsDirectory: download.EntryType == VolumeFileEntryType.Directory,
                        FileName: download.FileName)),
                cancellationToken);

            await unitOfWork.CommitAsync(cancellationToken);
        }

        return TypedResults.Empty;
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
