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
        ["images"] = typeof(ImageStreamManager),
        ["platforms"] = typeof(PlatformStreamManager),
        ["containers"] = typeof(ContainerStreamManager),
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
