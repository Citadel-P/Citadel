using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Builds;
using static Hosting.Common.Constants;

namespace Application.Services.SignalR;

public interface IBuildProjectStreamManager : IStreamGroupManager
{
    Task SendBuildProjectInfo(BuildProject project, string action = "update", BuildRun? latestRun = null);
}

public interface IBuildRunStreamManager : IStreamGroupManager
{
    Task SendBuildRunInfo(BuildRun run, string action = "update");
    Task SendBuildRunLogs(Guid runId, IReadOnlyList<BuildRunLogEntry> entries);
}

internal sealed class BuildProjectStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBuildProjectStreamManager
{
    public Task SendBuildProjectInfo(BuildProject project, string action = "update", BuildRun? latestRun = null)
        => dispatcher.SendBuildProjectInfo(project, action, latestRun);
}

internal sealed class BuildRunStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBuildRunStreamManager
{
    public Task SendBuildRunInfo(BuildRun run, string action = "update")
        => dispatcher.SendBuildRunInfo(run, action);

    public Task SendBuildRunLogs(Guid runId, IReadOnlyList<BuildRunLogEntry> entries)
    {
        if (entries.Count == 0 || !streams.ContainsKey(WellKnownSignalRGroups.BuildRunGroup(runId)))
            return Task.CompletedTask;

        return dispatcher.SendBuildRunLogs(runId, entries);
    }
}
