using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record ApplyStack(Guid Id, bool? Recreate = false) : IStreamCommand<StackStreamItem>;

internal sealed class ApplyStackHandler(
    IApplyStackService applyStackService,
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext) : IStreamCommandHandler<ApplyStack, StackStreamItem>
{
    public async IAsyncEnumerable<StackStreamItem> Handle(ApplyStack request, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var user = userContext.Current;
        var actorId = user.ActorId == Guid.Empty
            ? Constants.SystemId
            : user.ActorId;

        if (!user.IsAdmin)
        {
            var stack = await unitOfWork.Stacks.GetAsync(request.Id, cancellationToken);
            if (stack?.CurrentStackRelease is not null
                && !await unitOfWork.Platforms.CanAccessAsync(
                    user.UserId,
                    stack.CurrentStackRelease.PlatformId,
                    cancellationToken))
            {
                yield return StackStreamItem.FromStdErr(
                    "The stack platform does not exist or is not accessible.",
                    1);
                yield break;
            }
        }

        await foreach (var streamItem in applyStackService.ApplyAsync(
            request.Id,
            actorId,
            serviceNames: null,
            pullImages: false,
            recreate: request.Recreate == true,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            cancellationToken))
        {
            yield return streamItem;
        }
    }
}
