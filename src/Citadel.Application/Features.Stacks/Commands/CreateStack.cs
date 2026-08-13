using Domain;
using Application.Features.Stacks;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Activities;
using Domain.Entities.Stacks;
using Application.Services.Builds;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Application.Features.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Pipelines.Interfaces;
using Domain.Entities.ResourceBindings;
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
    StackDriftPolicy? DriftPolicy = null,
    IReadOnlyCollection<Guid>? TagIds = null,
    ActivitySourceResource? DuplicateSource = null) : ICommand<Result<Stack>>
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

internal sealed class CreateStackHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformHub,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService,
    IPermissionService permissionService) : ICommandHandler<CreateStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(CreateStack command, CancellationToken cancellationToken)
    {
        var user = userContext.Current;
        var actorId = user.ActorId;

        var platform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided platform does not exist."));
        }

        if (!user.IsAdmin
            && !await unitOfWork.Platforms.CanAccessAsync(user.ActorId, command.PlatformId, cancellationToken))
        {
            return Result.Failure<Stack>(new NotFoundError("The provided platform does not exist or is not accessible."));
        }

        if (await unitOfWork.Stacks.ExistsAsync(command.Name, cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError("Name already exists"));
        }

        var spec = BuildImageProvenance.Clear(command.Spec);
        var driftPolicy = command.DriftPolicy ?? StackDriftPolicy.Disabled;
        var entitlement = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            currentSpec: null,
            currentDriftPolicy: null,
            spec,
            driftPolicy,
            entitlementService,
            cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<Stack>(entitlementError);

        if (!IsCompatible(command.StackSource, spec))
        {
            return Result.Failure<Stack>(new BadRequestError("StackSource does not match the provided StackSpec."));
        }

        if (spec is GitStack gitSpec)
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

        var platformType = platform.PlatformDescriptor.Type;
        if (platformType is not PlatformType.Docker and not PlatformType.DockerSwarm)
        {
            return Result.Failure<Stack>(new BadRequestError("Stacks require a Docker Standalone or Docker Swarm platform."));
        }

        if (platformType == PlatformType.DockerSwarm)
        {
            var unsupported = SwarmStackConfigurationPolicy.GetIssues(spec, driftPolicy)
                .FirstOrDefault(static issue => issue.Severity == SwarmStackCompatibilitySeverity.Error);
            if (unsupported is not null)
                return Result.Failure<Stack>(new BadRequestError(unsupported.Message));
        }

        var duplicateSourceResult = await GetValidDuplicateSourceAsync(command.DuplicateSource, platformType, cancellationToken);
        if (!duplicateSourceResult.IsSuccess(out var duplicateSource))
        {
            return Result.Failure<Stack>(duplicateSourceResult.Errors);
        }

        IReadOnlyList<ResourceBinding> duplicateBindings = [];
        if (duplicateSource is not null)
        {
            var bindingsResult = await ResourceBindingsFeatureHelpers.GetDuplicateEntriesAsync(
                unitOfWork,
                permissionService,
                user,
                ResourceType.Stack,
                ResourceBindingScope.Stack,
                duplicateSource.ResourceId,
                cancellationToken);
            if (bindingsResult.IsFailure(out var bindingError, out var entries))
                return Result.Failure<Stack>(bindingError);

            duplicateBindings = entries ?? [];
        }

        var stack = Stack.Create(
            name: command.Name,
            createdByActorId: actorId,
            StackSource: command.StackSource,
            platformId: command.PlatformId,
            spec: spec,
            description: command.Description,
            driftPolicy: driftPolicy,
            platform: platform);

        var result = await unitOfWork.Stacks.AddAsync(stack, cancellationToken, command.TagIds, actorId);
        if (result == 0)
            return Result.Failure<Stack>(new BadRequestError("One or more tags do not exist."));

        await ResourceBindingsFeatureHelpers.CopyDuplicateEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            stack.Id,
            duplicateBindings,
            cancellationToken);

        var eventType = duplicateSource is null
            ? ActivityEventType.StackCreated
            : ActivityEventType.StackDuplicated;
        ActivityEventInfo info = duplicateSource is null
            ? new StackCreated(stack.ToSnapshot())
            : new StackDuplicated(stack.ToSnapshot(), duplicateSource);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: stack.Id,
            platformId: command.PlatformId,
            resourceName: stack.Name,
            eventType: eventType,
            status: ActivityStatus.Information,
            info: info);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        var updatedPlatform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(command.PlatformId, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        if (updatedPlatform is not null)
        {
            await platformHub.PushPlatformUpdate(updatedPlatform);
        }

        return stack;
    }

    private async Task<Result<ActivitySourceResource?>> GetValidDuplicateSourceAsync(
        ActivitySourceResource? source,
        PlatformType targetPlatformType,
        CancellationToken cancellationToken)
    {
        if (source is null)
            return Result.Success<ActivitySourceResource?>(null);

        if (source.ResourceType != ActivityResourceType.Stack)
            return Result.Failure<ActivitySourceResource?>(new BadRequestError("Duplicate source must be a stack."));

        var sourceStack = await unitOfWork.Stacks.GetAsync(source.ResourceId, cancellationToken);
        if (sourceStack is null)
            return Result.Failure<ActivitySourceResource?>(new NotFoundError("Duplicate source stack does not exist."));

        var user = userContext.Current;
        if (!user.IsAdmin && !await unitOfWork.Stacks.CanAccessAsync(user.ActorId, source.ResourceId, cancellationToken))
            return Result.Failure<ActivitySourceResource?>(new ForbiddenError("Missing permission [Read] on duplicate source stack."));

        if (sourceStack.CurrentStackRelease?.Platform?.PlatformDescriptor.Type != targetPlatformType)
            return Result.Failure<ActivitySourceResource?>(new BadRequestError("A stack duplicate must target the same platform type as its source."));

        return Result.Success<ActivitySourceResource?>(new ActivitySourceResource(
            ActivityResourceType.Stack,
            sourceStack.Id,
            sourceStack.Name));
    }

    private static bool IsCompatible(StackSource stackSource, StackSpec spec)
        => (stackSource, spec) switch
        {
            (StackSource.WebEditor, ManualStack) => true,
            (StackSource.Git, GitStack) => true,
            _ => false
        };

}
