using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;

namespace Application.Features.Alerters.Notifications;

internal static class AlertEventNotificationPayloadBuilder
{
    public static async Task<AlertEventNotificationPayload> BuildAsync(
        IUnitOfWork unitOfWork,
        IEnumerable<AlertEvent> alertEvents,
        CancellationToken cancellationToken)
    {
        var alertEventsByUser = new Dictionary<Guid, List<AlertEvent>>();

        foreach (var alertEvent in alertEvents.GroupBy(x => x.Id).Select(x => x.First()))
        {
            var userIds = await unitOfWork.AlertEvents.GetAuthorizedUserIdsAsync(
                alertEvent.Id,
                ResourceType.Alert,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken);

            foreach (var userId in userIds.Distinct())
            {
                if (!alertEventsByUser.TryGetValue(userId, out var userEvents))
                {
                    userEvents = [];
                    alertEventsByUser[userId] = userEvents;
                }

                userEvents.Add(alertEvent);
            }
        }

        var unresolvedCountsByUser = new Dictionary<Guid, int>();
        foreach (var userId in alertEventsByUser.Keys)
        {
            unresolvedCountsByUser[userId] = await unitOfWork.AlertEvents.CountAuthorizedUnresolvedAsync(
                userId,
                ResourceType.Alert,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken);
        }

        return new AlertEventNotificationPayload(
            alertEventsByUser.ToDictionary(kvp => kvp.Key, kvp => (IReadOnlyCollection<AlertEvent>)kvp.Value),
            unresolvedCountsByUser);
    }
}

internal sealed record AlertEventNotificationPayload(
    IReadOnlyDictionary<Guid, IReadOnlyCollection<AlertEvent>> AlertEventsByUser,
    IReadOnlyDictionary<Guid, int> UnresolvedCountsByUser);
