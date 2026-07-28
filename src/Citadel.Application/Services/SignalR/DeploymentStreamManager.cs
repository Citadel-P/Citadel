using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Deployments;

namespace Application.Services.SignalR;

internal interface IDeploymentStreamManager : IStreamGroupManager
{
    Task SendDeploymentInfo(Deployment deployment, string action = "update");
}

internal class DeploymentStreamManager(
    IApplicationHubDispatcher dispatcher,
    IPlatformStreamManager platformStreamManager) : BaseStreamManager<StreamContext>, IDeploymentStreamManager
{
    public Task SendDeploymentInfo(Deployment deployment, string action = "update")
    {
        var deploymentUpdate = streams.IsEmpty
            ? Task.CompletedTask
            : dispatcher.SendDeploymentInfo(deployment, action);
        var platformUpdate = platformStreamManager.RefreshPlatform(deployment.PlatformId);

        return Task.WhenAll(deploymentUpdate, platformUpdate);
    }
}
