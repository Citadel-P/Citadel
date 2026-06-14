using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Stacks;

namespace Application.Services.SignalR;

internal interface IStackStreamManager : IStreamGroupManager
{
    Task SendStackInfo(Stack stack, string action = "update");
}

internal class StackStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IStackStreamManager
{
    public Task SendStackInfo(Stack stack, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendStackInfo(stack, action);
    }
}