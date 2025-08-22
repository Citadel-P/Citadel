using System;
using System.Collections.Generic;
using System.Text;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Compose;

namespace Infrastructure.Connectors.AgentConnectors;

internal sealed class AgentComposeConnector : IComposeConnector
{
    public IAsyncEnumerable<ComposeDeploymentEvent> UpAsync(ComposeUpCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
