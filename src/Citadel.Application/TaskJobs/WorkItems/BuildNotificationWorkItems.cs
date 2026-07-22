using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;

namespace Application.TaskJobs.WorkItems;

internal sealed class BuildProjectNotificationWorkItem(
    IBuildProjectStreamManager streamManager,
    BuildProject project,
    string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendBuildProjectInfo(project, action);
}

internal sealed class BuildRunNotificationWorkItem(
    IBuildRunStreamManager streamManager,
    BuildRun run,
    string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendBuildRunInfo(run, action);
}

internal sealed class BuildAgentPoolNotificationWorkItem(
    IBuildAgentPoolStreamManager streamManager,
    BuildAgentPool pool,
    string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendBuildAgentPoolInfo(pool, action);
}
