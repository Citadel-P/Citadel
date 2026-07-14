using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Automation;

namespace Application.Services.SignalR;

public interface IAutomationActionStreamManager : IStreamGroupManager
{
    Task SendAutomationActionInfo(AutomationAction action, string actionName = "update");
}

internal sealed class AutomationActionStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IAutomationActionStreamManager
{
    public Task SendAutomationActionInfo(AutomationAction action, string actionName = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendAutomationActionInfo(action, actionName);
    }
}
