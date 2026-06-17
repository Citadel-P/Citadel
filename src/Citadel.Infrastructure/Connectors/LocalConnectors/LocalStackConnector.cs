using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Hosting.DockerClient.Services;
using Infrastructure.Connectors.Mappers;
using System.Runtime.CompilerServices;

namespace Infrastructure.Connectors.LocalConnectors;

internal sealed class LocalStackConnector(IStackService stackService) : IStackConnector
{
    public async IAsyncEnumerable<StackApplyResult> StackApplyAsync(StackApplyCommand applyCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in stackService.ApplyStreamAsync(applyCommand.ToCommand(), cancellationToken))
        {
            yield return result.Map();
        }
    }
}
