using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using LightResults;
using System;
using System.Collections.Generic;
using System.Text;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentStackConnector : IStackConnector
{
    public IAsyncEnumerable<StackApplyResult> StackApplyAsync(StackApplyCommand applyCommand, CancellationToken cancellationToken)
    {
        // Will implement this later, do not implement this for now
        throw new NotImplementedException();
    }
}
