using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IDeploymentStreamManager : IStreamGroupManager
{
    Task SendDeploymentInfo(Deployment deployment);
    Task SendDeploymentsInfo(IEnumerable<Deployment> deployments);
}

internal class DeploymentStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IDeploymentStreamManager
{
    public Task SendDeploymentInfo(Deployment deployment)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendDeploymentInfo(deployment);
    }

    public Task SendDeploymentsInfo(IEnumerable<Deployment> deployments)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendDeploymentsInfo(deployments);
    }
}

