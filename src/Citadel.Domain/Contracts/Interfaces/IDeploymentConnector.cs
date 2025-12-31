using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Images;
using System;
using System.Collections.Generic;
using System.Text;

namespace Domain.Contracts.Interfaces;

public interface IDeploymentConnector
{
    IAsyncEnumerable<DeploymentStreamItem> ApplyDeploymentAsync(PullImageCommand pullImageCommand, CancellationToken cancellationToken);

}
