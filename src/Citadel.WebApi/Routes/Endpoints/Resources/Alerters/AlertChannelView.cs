using Domain;
using Domain.Entities.Alerts;

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

public sealed record AlertChannelsView(IEnumerable<AlertChannelView> Channels)
{
    internal static AlertChannelsView Map(IEnumerable<AlertChannel> channels)
    {
        return new(channels.Select(AlertChannelView.Map));
    }
}