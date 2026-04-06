using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, ResourceAction.Update)]
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

internal sealed class RenameStackHandler(IUnitOfWork unitOfWork) : ICommandHandler<RenameStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(RenameStack command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack == null || stack.CurrentStackRelease == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided stack does not exist."));
        }

        if (await unitOfWork.Stacks.ExistsAsync(command.Id, command.Name, cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError("Name already exists"));
        }

        stack.UpdateDetails(name: command.Name);

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }
}
