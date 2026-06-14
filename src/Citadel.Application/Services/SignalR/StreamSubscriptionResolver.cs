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
        ["stack"] = typeof(StackStreamManager),
        ["stacks"] = typeof(StackStreamManager),
        ["images"] = typeof(ImageStreamManager),
        ["activity"] = typeof(ActivityStreamManager),
        ["platforms"] = typeof(PlatformStreamManager),
        ["containers"] = typeof(ContainerStreamManager),
        ["deployment"] = typeof(DeploymentStreamManager),
        ["deployments"] = typeof(DeploymentStreamManager),
        ["git-repo"] = typeof(GitRepositoryStreamManager),
        ["git-repositories"] = typeof(GitRepositoryStreamManager),
        ["alert-events"] = typeof(AlertEventStreamManager),
        ["docker-daemon"] = typeof(DockerDaemonStreamManager),
    };

    public IStreamGroupManager Resolve(string groupId)
    {
        var prefix = groupId.Split(':')[0];
        if (!handlers.TryGetValue(prefix, out var handlerType))
        {
            throw new InvalidOperationException($"Unknown stream group: {prefix}");
        }

        return (IStreamGroupManager)provider.GetRequiredService(handlerType);
    }
}
