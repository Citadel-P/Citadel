using System.ComponentModel;
using Application.Features.Platforms.Commands;
using Domain.Entities.Platforms;
using Hosting.Common.MergePatch;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints;

public static class PlatformMetadata
{
    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> PatchMetadata(
        IMediator mediator,
        [FromRoute][Description("Platform ID")] Guid id,
        PatchResourceMetadataDocument patchInput,
        CancellationToken cancellationToken)
    {
        var mapped = patchInput.Map<PatchResourceMetadata, Platform>();
        var result = await mediator.Send(new PatchPlatformMetadata(id, mapped), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }

    public static async Task<Results<Ok<PlatformView>, ProblemHttpResult>> Rename(
        IMediator mediator,
        [FromBody] RenameResource renameResource,
        CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new RenamePlatform(renameResource.Id, renameResource.Name), cancellationToken);
        return EndpointHandlers.HandleResult(result, PlatformView.Map);
    }
}
