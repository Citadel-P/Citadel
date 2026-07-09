using Citadel.Stacks.V1;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Grpc.Core;
using Infrastructure.Connectors.Mappers;
using Infrastructure.Repositories;
using System.Runtime.CompilerServices;

namespace Infrastructure.Connectors.AgentConnectors;

internal class AgentStackConnector(IGrpcClientFactory clientFactory) : IStackConnector
{
    public async IAsyncEnumerable<StackApplyResult> StackApplyAsync(
        StackApplyCommand applyCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var stackClient = clientFactory.GetStackClient(applyCommand.PlatformAddress);
        using var call = stackClient.Apply(applyCommand.ToAgentRequest(), cancellationToken: cancellationToken);

        await foreach (StackApplyResponse result in call.ResponseStream.ReadAllAsync(cancellationToken))
            yield return result.Map();
    }
}
