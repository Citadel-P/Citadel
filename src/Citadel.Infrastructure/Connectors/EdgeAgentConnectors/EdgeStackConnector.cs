using System.Runtime.CompilerServices;
using Citadel.Stacks.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeStackConnector(IEdgeAgentCommandRouter commandRouter) : IStackConnector
{
    public async IAsyncEnumerable<StackApplyResult> StackApplyAsync(
        StackApplyCommand applyCommand,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(applyCommand.PlatformAddress, out var platformId, out var addressError))
        {
            yield return new StackApplyResult(Domain.StackApplyEventType.StdErr, addressError?.Message ?? "Edge Agent platform address is invalid.", null);
            yield break;
        }

        await foreach (var item in commandRouter.SendServerStreamAsync(
                           platformId,
                           EdgeAgentCommandKind.StackApplyStream,
                           applyCommand.ToAgentRequest().ToByteArray(),
                           TimeSpan.FromHours(2),
                           correlationId: null,
                           cancellationToken))
        {
            if (item.ErrorMessage is not null)
            {
                yield return new StackApplyResult(Domain.StackApplyEventType.StdErr, item.ErrorMessage, null);
                yield break;
            }

            if (item.Completed)
            {
                yield break;
            }

            if (item.Payload is { Length: > 0 })
            {
                yield return StackApplyResponse.Parser.ParseFrom(item.Payload).Map();
            }
        }
    }
}
