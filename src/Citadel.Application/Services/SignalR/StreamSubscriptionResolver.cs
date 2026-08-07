using Application.Services.Abstractions;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.SignalR;

public interface IStreamSubscriptionResolver
{
    IStreamGroupManager Resolve(string groupId);
}

internal sealed class StreamSubscriptionResolver(IServiceProvider provider) : IStreamSubscriptionResolver
{
    private readonly Dictionary<string, Type> handlers = new()
    {
        ["container-info"] = typeof(ContainerInfoStreamManager),
        ["container-log"] = typeof(ContainerLogStreamManager),
        ["container-exec"] = typeof(ExecSessionManager),
        ["stack-info"] = typeof(StackInfoStreamManager),
        ["stack-log"] = typeof(StackLogStreamManager),
        ["stack"] = typeof(StackStreamManager),
        ["stacks"] = typeof(StackStreamManager),
        ["images"] = typeof(ImageStreamManager),
        ["activity"] = typeof(ActivityStreamManager),
        ["platforms"] = typeof(PlatformStreamManager),
        ["containers"] = typeof(ContainerStreamManager),
        ["deployment"] = typeof(DeploymentStreamManager),
        ["deployments"] = typeof(DeploymentStreamManager),
        ["swarm-service"] = typeof(SwarmServiceStreamManager),
        ["swarm-services"] = typeof(SwarmServiceStreamManager),
        ["git-repo"] = typeof(GitRepositoryStreamManager),
        ["git-repositories"] = typeof(GitRepositoryStreamManager),
        ["backup-repository"] = typeof(BackupRepositoryStreamManager),
        ["backup-repositories"] = typeof(BackupRepositoryStreamManager),
        ["backup-policy"] = typeof(BackupPolicyStreamManager),
        ["backup-policies"] = typeof(BackupPolicyStreamManager),
        ["backup-run"] = typeof(BackupRunStreamManager),
        ["backup-runs"] = typeof(BackupRunStreamManager),
        ["backup-restore-run"] = typeof(BackupRestoreRunStreamManager),
        ["backup-restore-runs"] = typeof(BackupRestoreRunStreamManager),
        ["build-project"] = typeof(BuildProjectStreamManager),
        ["build-projects"] = typeof(BuildProjectStreamManager),
        ["build-agent-pool"] = typeof(BuildAgentPoolStreamManager),
        ["build-agent-pools"] = typeof(BuildAgentPoolStreamManager),
        ["build-run"] = typeof(BuildRunStreamManager),
        ["build-runs"] = typeof(BuildRunStreamManager),
        ["automation-action"] = typeof(AutomationActionStreamManager),
        ["automation-actions"] = typeof(AutomationActionStreamManager),
        ["alert-events"] = typeof(AlertEventStreamManager),
        ["docker-daemon"] = typeof(DockerDaemonStreamManager),
    };

    public IStreamGroupManager Resolve(string groupId)
    {
        var separatorIndex = groupId.IndexOf(':');
        var prefix = separatorIndex < 0 ? groupId : groupId[..separatorIndex];
        if (!handlers.TryGetValue(prefix, out var handlerType))
        {
            throw new InvalidOperationException($"Unknown stream group: {prefix}");
        }

        return (IStreamGroupManager)provider.GetRequiredService(handlerType);
    }
}
