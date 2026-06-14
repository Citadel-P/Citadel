using Application.Services;
using Domain.Contracts.Resources.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyStack(Guid Id) : IStreamCommand<StackStreamItem>;

internal sealed class ApplyStackHandler(IApplyStackService applyStackService) : IStreamCommandHandler<ApplyStack, StackStreamItem>
{
    public async IAsyncEnumerable<StackStreamItem> Handle(ApplyStack request, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var streamItem in applyStackService.ApplyAsync(request.Id, cancellationToken))
        {
            yield return streamItem;
        }
    }
}