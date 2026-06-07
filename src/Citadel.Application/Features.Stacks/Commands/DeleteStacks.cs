using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Execute)]
public sealed record DeleteStacks(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteStacksHandler(
    IUnitOfWork unitOfWork, 
    IUserContextAccessor userContext) : ICommandHandler<DeleteStacks, Result>
{
    public async ValueTask<Result> Handle(DeleteStacks command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var stacks = await unitOfWork.Stacks.GetAllAsync(command.Ids, cancellationToken);
        if (stacks == null || !stacks.Any())
        {
            return Result.Failure(new NotFoundError("No stacks found matching the provided IDs."));
        }

        await unitOfWork.Stacks.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}