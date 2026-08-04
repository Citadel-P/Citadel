using Application.Features.Swarm.Queries;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources.Swarm;

namespace WebApi.Routes.Endpoints;

public static class SwarmInventory
{
    public static async Task<Results<Ok<SwarmServicesView>, ProblemHttpResult>> ListServices(IMediator mediator, Guid platformId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmServices(platformId), ct), SwarmServicesView.Map);
    public static async Task<Results<Ok<SwarmServiceView>, ProblemHttpResult>> GetService(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmService(platformId, resourceId), ct), SwarmServiceView.Map);
    public static async Task<Results<Ok<SwarmTasksView>, ProblemHttpResult>> ListTasks(IMediator mediator, Guid platformId, int limit = 50, CancellationToken ct = default) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmTasks(platformId, limit), ct), SwarmTasksView.Map);
    public static async Task<Results<Ok<SwarmTaskView>, ProblemHttpResult>> GetTask(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmTask(platformId, resourceId), ct), SwarmTaskView.Map);
    public static async Task<Results<Ok<SwarmNetworksView>, ProblemHttpResult>> ListNetworks(IMediator mediator, Guid platformId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmNetworks(platformId), ct), SwarmNetworksView.Map);
    public static async Task<Results<Ok<SwarmNetworkView>, ProblemHttpResult>> GetNetwork(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmNetwork(platformId, resourceId), ct), SwarmNetworkView.Map);
    public static async Task<Results<Ok<SwarmSecretsView>, ProblemHttpResult>> ListSecrets(IMediator mediator, Guid platformId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmSecrets(platformId), ct), SwarmSecretsView.Map);
    public static async Task<Results<Ok<SwarmSecretView>, ProblemHttpResult>> GetSecret(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmSecret(platformId, resourceId), ct), SwarmSecretView.Map);
    public static async Task<Results<Ok<SwarmConfigsView>, ProblemHttpResult>> ListConfigs(IMediator mediator, Guid platformId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmConfigs(platformId), ct), SwarmConfigsView.Map);
    public static async Task<Results<Ok<SwarmConfigView>, ProblemHttpResult>> GetConfig(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmConfig(platformId, resourceId), ct), SwarmConfigView.Map);
}
