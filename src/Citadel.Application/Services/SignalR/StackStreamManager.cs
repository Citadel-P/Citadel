using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Stacks;

namespace Application.Services.SignalR;

internal interface IStackStreamManager : IStreamGroupManager
{
    Task SendStackInfo(Stack stack, string action = "update");
}

internal class StackStreamManager(
    IApplicationHubDispatcher dispatcher,
    IPlatformStreamManager platformStreamManager) : BaseStreamManager<StreamContext>, IStackStreamManager
{
    public Task SendStackInfo(Stack stack, string action = "update")
    {
        var stackUpdate = streams.IsEmpty
            ? Task.CompletedTask
            : dispatcher.SendStackInfo(stack, action);
        var platformId = stack.CurrentStackRelease?.PlatformId;
        if (platformId is null)
        {
            return stackUpdate;
        }

        return Task.WhenAll(stackUpdate, platformStreamManager.RefreshPlatform(platformId.Value));
    }
}
