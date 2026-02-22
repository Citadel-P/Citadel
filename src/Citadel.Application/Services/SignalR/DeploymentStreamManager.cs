using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Deployments;
using System;

namespace Application.Services.SignalR;

internal interface IDeploymentStreamManager : IStreamGroupManager
{
    Task SendDeploymentInfo(Deployment deployment, string action = "update");
}

internal class DeploymentStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IDeploymentStreamManager
{
    public Task SendDeploymentInfo(Deployment deployment, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendDeploymentInfo(deployment, action);
    }
}

