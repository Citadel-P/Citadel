using Application.Features.Swarm.Queries;
using Application.Permissions;
using Hosting.Extensions;
using Mediator;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints.Resources.Swarm;

namespace WebApi.Routes.Endpoints;

public static class SwarmInventory
{
    public static async Task<Results<Ok<SwarmOverviewView>, ProblemHttpResult>> GetOverview(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken ct)
    {
        var result = await mediator.Send(new GetSwarmOverview(platformId), ct);
        return await EndpointHandlers.HandleResult(
            result,
            permissionEvaluator,
            (value, evaluator) => SwarmOverviewView.Map(platformId, value, evaluator));
    }

    public static async Task<Results<Ok<SwarmServicesView>, ProblemHttpResult>> ListServices(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmServices(platformId), ct),
            permissionEvaluator,
            (values, evaluator) => SwarmServicesView.Map(values, platformId, evaluator));
    public static async Task<Results<Ok<SwarmServiceView>, ProblemHttpResult>> GetService(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmService(platformId, resourceId), ct),
            permissionEvaluator,
            SwarmServiceView.Map);
    public static async Task<Results<Ok<SwarmServiceInspectView>, ProblemHttpResult>> InspectService(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new InspectSwarmService(platformId, resourceId), ct), SwarmServiceInspectView.Map);
    public static async Task<Results<Ok<SwarmLogsView>, ProblemHttpResult>> GetServiceLogs(IMediator mediator, Guid platformId, string resourceId, int tail = 100, CancellationToken ct = default) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmServiceLogs(platformId, resourceId, tail), ct), SwarmLogsView.Map);
    public static async Task<Results<Ok<SwarmTasksView>, ProblemHttpResult>> ListTasks(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        int limit = 50,
        string? serviceId = null,
        CancellationToken ct = default) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmTasks(platformId, limit, serviceId), ct),
            permissionEvaluator,
            (values, evaluator) => SwarmTasksView.Map(values, platformId, evaluator));
    public static async Task<Results<Ok<SwarmTaskView>, ProblemHttpResult>> GetTask(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmTask(platformId, resourceId), ct),
            permissionEvaluator,
            SwarmTaskView.Map);
    public static async Task<Results<Ok<SwarmTaskInspectView>, ProblemHttpResult>> InspectTask(IMediator mediator, Guid platformId, string resourceId, CancellationToken ct) =>
        EndpointHandlers.HandleResult(await mediator.Send(new InspectSwarmTask(platformId, resourceId), ct), SwarmTaskInspectView.Map);
    public static async Task<Results<Ok<SwarmTaskStatsView>, ProblemHttpResult>> GetTaskStats(
        IMediator mediator,
        Guid platformId,
        string resourceId,
        int hours = 24,
        CancellationToken ct = default) =>
        EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmTaskStats(platformId, resourceId, hours), ct),
            SwarmTaskStatsView.Map);
    public static async Task<Results<Ok<SwarmTaskTerminalView>, ProblemHttpResult>> GetTaskTerminalTarget(
        IMediator mediator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmTaskTerminalTarget(platformId, resourceId), ct),
            static dockerContainerId => new SwarmTaskTerminalView(dockerContainerId));
    public static async Task<Results<Ok<SwarmLogsView>, ProblemHttpResult>> GetTaskLogs(IMediator mediator, Guid platformId, string resourceId, int tail = 100, CancellationToken ct = default) =>
        EndpointHandlers.HandleResult(await mediator.Send(new GetSwarmTaskLogs(platformId, resourceId, tail), ct), SwarmLogsView.Map);
    public static async Task<Results<Ok<SwarmNetworksView>, ProblemHttpResult>> ListNetworks(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmNetworks(platformId), ct),
            permissionEvaluator,
            (values, evaluator) => SwarmNetworksView.Map(values, platformId, evaluator));
    public static async Task<Results<Ok<SwarmNetworkView>, ProblemHttpResult>> GetNetwork(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmNetwork(platformId, resourceId), ct),
            permissionEvaluator,
            SwarmNetworkView.Map);
    public static async Task<Results<Ok<SwarmSecretsView>, ProblemHttpResult>> ListSecrets(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmSecrets(platformId), ct),
            permissionEvaluator,
            (values, evaluator) => SwarmSecretsView.Map(values, platformId, evaluator));
    public static async Task<Results<Ok<SwarmSecretView>, ProblemHttpResult>> GetSecret(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmSecret(platformId, resourceId), ct),
            permissionEvaluator,
            SwarmSecretView.Map);
    public static async Task<Results<NoContent, ProblemHttpResult>> CreateSecret(
        IMediator mediator,
        Guid platformId,
        [FromBody] CreateSwarmSecretInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToCommand(platformId), ct));
    public static async Task<Results<NoContent, ProblemHttpResult>> UpdateSecretLabels(
        IMediator mediator,
        Guid platformId,
        string resourceId,
        [FromBody] UpdateSwarmResourceLabelsInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToSecretCommand(platformId, resourceId), ct));
    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteSecrets(
        IMediator mediator,
        Guid platformId,
        [FromBody] DeleteSwarmResourcesInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToSecretCommand(platformId), ct));
    public static async Task<Results<Ok<SwarmConfigsView>, ProblemHttpResult>> ListConfigs(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmConfigs(platformId), ct),
            permissionEvaluator,
            (values, evaluator) => SwarmConfigsView.Map(values, platformId, evaluator));
    public static async Task<Results<Ok<SwarmConfigView>, ProblemHttpResult>> GetConfig(
        IMediator mediator,
        IPermissionEvaluator permissionEvaluator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        await EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmConfig(platformId, resourceId), ct),
            permissionEvaluator,
            SwarmConfigView.Map);
    public static async Task<Results<Ok<SwarmConfigDataView>, ProblemHttpResult>> GetConfigData(
        IMediator mediator,
        Guid platformId,
        string resourceId,
        CancellationToken ct) =>
        EndpointHandlers.HandleResult(
            await mediator.Send(new GetSwarmConfigData(platformId, resourceId), ct),
            static content => new SwarmConfigDataView(content));
    public static async Task<Results<NoContent, ProblemHttpResult>> CreateConfig(
        IMediator mediator,
        Guid platformId,
        [FromBody] CreateSwarmConfigInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToCommand(platformId), ct));
    public static async Task<Results<NoContent, ProblemHttpResult>> UpdateConfigLabels(
        IMediator mediator,
        Guid platformId,
        string resourceId,
        [FromBody] UpdateSwarmResourceLabelsInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToConfigCommand(platformId, resourceId), ct));
    public static async Task<Results<NoContent, ProblemHttpResult>> DeleteConfigs(
        IMediator mediator,
        Guid platformId,
        [FromBody] DeleteSwarmResourcesInput request,
        CancellationToken ct) =>
        EndpointHandlers.HandleResultForNoContent(
            await mediator.Send(request.ToConfigCommand(platformId), ct));
}
