using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;

namespace Application.Features.Deployments.Notifications;

internal sealed class ActivityNotificationWorkItem(IActivityStreamManager activityStream, ActivityEvent activityEvent) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
    => activityStream.SendActivityInfo(activityEvent);
}

