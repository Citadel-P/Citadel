using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Application.Services;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record RenameStack(Guid Id, string Name) : ICommand<Result<Stack>>
{
    internal sealed class Validator : AbstractValidator<RenameStack>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameStackHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<RenameStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(RenameStack command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack == null || stack.CurrentStackRelease == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided stack does not exist."));
        }

        if (await unitOfWork.Stacks.ExistsAsync(command.Id, command.Name, cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError("Name already exists"));
        }

        var currentProjectName = StackProjectNameResolver.Resolve(stack);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: stack.Id,
            platformId: stack.CurrentStackRelease.PlatformId,
            resourceName: command.Name,
            eventType: ActivityEventType.StackRenamed,
            status: ActivityStatus.Success,
            info: new StackRenamed(stack.Name, command.Name)
           );

        stack.PinCurrentStackProjectName(currentProjectName);
        stack.UpdateDetails(name: command.Name);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }
}
