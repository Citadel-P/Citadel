using Application.Services;
using Domain.Contracts.Resources.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyStack(Guid Id) : IStreamCommand<StackStreamItem>;

internal sealed class ApplyStackHandler(
    IApplyStackService applyStackService,
    IUserContextAccessor userContext) : IStreamCommandHandler<ApplyStack, StackStreamItem>
{
    public async IAsyncEnumerable<StackStreamItem> Handle(ApplyStack request, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;

        await foreach (var streamItem in applyStackService.ApplyAsync(
            request.Id,
            actorId,
            serviceNames: null,
            pullImages: false,
            cancellationToken))
        {
            yield return streamItem;
        }
    }
}
