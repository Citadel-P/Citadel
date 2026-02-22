using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Activities;

namespace Application.Services.SignalR;

internal interface IActivityStreamManager : IStreamGroupManager
{
    Task SendActivityInfo(ActivityEvent activity);
}

internal class ActivityStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IActivityStreamManager
{
    public Task SendActivityInfo(ActivityEvent activity)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendActivityInfo(activity);
    }
}

