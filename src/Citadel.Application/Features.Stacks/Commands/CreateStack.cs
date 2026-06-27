using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record CreateStack(
    string Name,
    Guid PlatformId,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    StackDriftPolicy? DriftPolicy = null) : ICommand<Result<Stack>>
{
    internal sealed class Validator : AbstractValidator<CreateStack>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);
            RuleFor(x => x.Spec).NotNull();
            RuleFor(x => x.DriftPolicy).Must(BeSafePolicy)
                .WithMessage("RemoveExtraContainers can only be enabled when drift mode is AutoFix.");
        }

        private static bool BeSafePolicy(StackDriftPolicy? policy)
            => policy is null
            || !policy.RemoveExtraContainers
            || policy.Mode == StackDriftMode.AutoFix;
    }
}

internal sealed class CreateStackHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<CreateStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(CreateStack command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        if (await unitOfWork.Stacks.ExistsAsync(command.Name, cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError("Name already exists"));
        }

        if (!IsCompatible(command.StackSource, command.Spec))
        {
            return Result.Failure<Stack>(new BadRequestError("StackSource does not match the provided StackSpec."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided platform does not exist."));
        }

        if (command.Spec is GitStack gitSpec)
        {
            var validationError = GitStackSpecValidation.Validate(gitSpec);
            if (validationError is not null)
            {
                return Result.Failure<Stack>(new BadRequestError(validationError));
            }

            var gitRepository = await unitOfWork.GitRepositories.GetAsync(gitSpec.GitRepoId, cancellationToken);
            if (gitRepository == null)
            {
                return Result.Failure<Stack>(new NotFoundError("The provided git repository does not exist."));
            }
        }

        // Add DockerStack
        var stack = Stack.Create(
            name: command.Name,
            createdByActorId: actorId,
            StackSource: command.StackSource,
            platformId: command.PlatformId,
            spec: command.Spec,
            description: command.Description,
            driftPolicy: command.DriftPolicy);

        await unitOfWork.Stacks.AddAsync(stack, cancellationToken);

        // Add activity
        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: stack.Id,
            platformId: command.PlatformId,
            resourceName: stack.Name,
            eventType: ActivityEventType.StackCreated,
            status: ActivityStatus.Information,
            info: new StackCreated(stack.ToSnapshot())
            );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }

    private static bool IsCompatible(StackSource stackSource, StackSpec spec)
        => (stackSource, spec) switch
        {
            (StackSource.WebEditor, ManualStack) => true,
            (StackSource.Git, GitStack) => true,
            _ => false
        };
}
