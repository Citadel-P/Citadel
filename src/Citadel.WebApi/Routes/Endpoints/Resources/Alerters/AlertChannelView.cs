using Application.Permissions;
using Domain;
using Domain.Entities.Alerts;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertChannelView(
    Guid Id,
    string Name,
    AlertDestination AlertDestination,
    string Url,
    bool IsActive,
    Guid CreatedByActorId,
    DateTime CreatedAt)
{
    internal static AlertChannelView Map(AlertChannel channel)
    {
        return new(
            channel.Id,
            channel.Name,
            channel.AlertDestination,
            channel.Url,
            channel.IsActive,
            channel.CreatedByActorId,
            channel.CreatedAt);
    }
}

public sealed record AlertChannelsView(IEnumerable<AlertChannelView> Channels, ResourceCapabilities Capabilities)
{
    internal static async Task<AlertChannelsView> Map(IEnumerable<AlertChannel> channels, IPermissionEvaluator permissionEvaluator)
    {
        var list = channels as AlertChannel[] ?? [.. channels];
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.AlertChannel);
        if (list.Length == 0)
            return new AlertChannelsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var views = list.Select(AlertChannelView.Map).ToArray();
        return new AlertChannelsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}