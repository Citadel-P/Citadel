using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.SwarmServices;

namespace Application.Services.SignalR;

internal interface ISwarmServiceStreamManager : IStreamGroupManager
{
    Task SendSwarmServiceInfo(SwarmService service, string action = "update");
}

internal sealed class SwarmServiceStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, ISwarmServiceStreamManager
{
    public Task SendSwarmServiceInfo(SwarmService service, string action = "update") =>
        streams.IsEmpty
            ? Task.CompletedTask
            : dispatcher.SendSwarmServiceInfo(service, action);
}
