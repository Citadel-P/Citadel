using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;

namespace Application.Services;

internal class ActivityNotification(ActivityEvent activity) : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
    {
        return activity.ResourceType switch 
        {
            ActivityResourceType.Deployment => EnqueueDeploymentNotification(cancellationToken),
            _ => Task.CompletedTask
        };
    }

    private Task EnqueueDeploymentNotification(CancellationToken cancellationToken)
    {
        // Todo: notify clients
        return Task.CompletedTask;
    }
}