using Application.Features.Platforms.Queries;
using Application.Services.Abstractions;
using Mediator;
using Microsoft.AspNetCore.Authorization;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Hubs;

public interface ITypedPlatformHub
{
    Task PlatformsUpdated(IEnumerable<PlatformView> platforms);
    Task PlatformStatsUpdated(PlatformStatsBatchView platform);
    Task PlatformUpdated(PlatformView platform);
    Task PlatformsDeleted(Guid platformId);
}

[Authorize]
internal sealed class PlatformHub(IMediator mediator, ISignalRConnectionTracker connectionTracker) : HubBase<ITypedPlatformHub>(connectionTracker)
{
    public async Task<PlatformsView> GetPlatforms()
    {
        var result = await mediator.Send(new GetPlatforms());
        if (result.IsSuccess(out var platforms))
        {
            return PlatformsView.Map(platforms);
        }
        else return new PlatformsView([]);
    }
}
