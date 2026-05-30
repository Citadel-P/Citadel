using Application.Features.Volumes.Queries;
using Application.Permissions;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using System.ComponentModel;
using WebApi.Routes.Endpoints.Resources.Volumes;

namespace WebApi.Routes.Endpoints;

public static class Volumes
{
    public static async Task<Results<Ok<VolumesView>, ProblemHttpResult>> List(IMediator mediator, IPermissionEvaluator permissionEvaluator, [Description("The platform id")] Guid platformId, [AsParameters] ListVolumesRequest listNetworksRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(platformId), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, VolumesView.Map);
    }

    public static async Task<Results<Ok<DockerVolumeResultView>, ProblemHttpResult>> Create(IMediator mediator, IPermissionEvaluator permissionEvaluator, [FromBody] CreateVolumeInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerVolumeResultView.Map);
    }

    public static async Task<Results<Ok<DockerVolumeResultView>, ProblemHttpResult>> Inspect(IMediator mediator, IPermissionEvaluator permissionEvaluator, Guid platformId, string name, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectVolume(platformId, name), cancellationToken);
        return await EndpointHandlers.HandleResult(result, permissionEvaluator, DockerVolumeResultView.Map);
    }

    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteVolumesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
