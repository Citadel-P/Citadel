using Domain.Contracts.Resources.Stacks;

namespace Domain.Contracts.Interfaces;

public interface IStackConnector
{
    IAsyncEnumerable<StackApplyResult> StackApplyAsync(StackApplyCommand applyCommand, CancellationToken cancellationToken);
}
