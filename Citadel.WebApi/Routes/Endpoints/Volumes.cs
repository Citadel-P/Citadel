using System.ComponentModel;
using Application.Features.Volumes.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Volumes;

namespace WebApi.Routes.Endpoints;

public static class Volumes
{
    public static async Task<Results<Ok<VolumesView>, ProblemHttpResult>> List(IMediator mediator, [Description("The platform id")] Guid id, [AsParameters] ListVolumesRequest listNetworksRequest, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(listNetworksRequest.ToQuery(id), cancellationToken);
        return EndpointHandlers.HandleResult(result, VolumeView.Map);
    }
    public static async Task<Results<Ok<InspectVolumeView>, ProblemHttpResult>> Inspect(IMediator mediator, Guid platformId, string name, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(new InspectVolume(platformId, name), cancellationToken);
        return EndpointHandlers.HandleResult(result, InspectVolumeView.Map);
    }
    public static async Task<Results<NoContent, ProblemHttpResult>> Delete(IMediator mediator, [FromBody] DeleteVolumesInput request, CancellationToken cancellationToken)
    {
        var result = await mediator.Send(request.ToCommand(), cancellationToken);
        return EndpointHandlers.HandleResultForNoContent(result);
    }
}
