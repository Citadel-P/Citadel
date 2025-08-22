using System;
using System.Collections.Generic;
using System.Text;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Compose;

namespace Infrastructure.Connectors.LocalConnectors;

internal sealed class LocalComposeConnector : IComposeConnector
{
    public IAsyncEnumerable<ComposeDeploymentEvent> UpAsync(ComposeUpCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
