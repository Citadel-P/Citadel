using Application.Permissions;
using Domain;
using Domain.Entities.Alerts;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertChannelView(
    Guid Id,
    string Name,
    AlertDestination AlertDestination,
    string Url,
    bool IsActive,
    Guid CreatedByActorId,
    DateTime CreatedAt,
    ResourceCapabilities? Capabilities = null)
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
            channel.CreatedAt,
            null);
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

        var ids = list.Select(static channel => channel.Id).ToArray();
        var permissions = await permissionEvaluator.EvaluateAsync(ids, ResourceType.AlertChannel);
        var views = new AlertChannelView[list.Length];

        for (var i = 0; i < list.Length; i++)
        {
            var channel = list[i];
            permissions.TryGetValue(channel.Id, out var metadata);
            views[i] = AlertChannelView.Map(channel) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(
                    metadata == default ? PermissionMetadata.Empty : metadata)
            };
        }

        return new AlertChannelsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}
