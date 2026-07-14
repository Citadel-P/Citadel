using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Automation;

namespace Application.TaskJobs.WorkItems;

internal sealed class AutomationActionNotificationWorkItem(
    IAutomationActionStreamManager streamManager,
    AutomationAction action,
    string actionName = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendAutomationActionInfo(action, actionName);
}
