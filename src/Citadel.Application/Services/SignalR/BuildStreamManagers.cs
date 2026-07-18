using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Builds;
using static Hosting.Common.Constants;

namespace Application.Services.SignalR;

public interface IBuildProjectStreamManager : IStreamGroupManager
{
    Task SendBuildProjectInfo(BuildProject project, string action = "update");
}

public interface IBuildRunStreamManager : IStreamGroupManager
{
    Task SendBuildRunInfo(BuildRun run, string action = "update");
}

internal sealed class BuildProjectStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBuildProjectStreamManager
{
    public Task SendBuildProjectInfo(BuildProject project, string action = "update")
    {
        if (!streams.ContainsKey(WellKnownSignalRGroups.BuildProjectGroup(project.Id)) &&
            !streams.ContainsKey(WellKnownSignalRGroups.BuildProjectsGroup))
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBuildProjectInfo(project, action);
    }
}

internal sealed class BuildRunStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBuildRunStreamManager
{
    public Task SendBuildRunInfo(BuildRun run, string action = "update")
    {
        if (!streams.ContainsKey(WellKnownSignalRGroups.BuildRunGroup(run.Id)) &&
            !streams.ContainsKey(WellKnownSignalRGroups.BuildRunsGroup(run.BuildProjectId)))
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBuildRunInfo(run, action);
    }
}
