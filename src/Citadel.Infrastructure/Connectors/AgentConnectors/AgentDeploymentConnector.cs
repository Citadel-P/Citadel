using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Images;
using LightResults;
using System;
using System.Collections.Generic;
using System.Text;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentDeploymentConnector : IDeploymentConnector
{
    Task<Result<string>> IDeploymentConnector.ApplyDeploymentAsync(ApplyDeploymentCommand applyDeployment, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
