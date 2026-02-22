using Domain;
using Domain.Entities.Alerts;

namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record AlertChannelView(
    Guid Id,
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
            channel.AlertDestination,
            channel.Url,
            channel.IsActive,
            channel.CreatedByActorId,
            channel.CreatedAt);
    }
}
