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
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using System.Text.Json;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record PatchStack(Guid Id, JsonMergePatchDocument<StackPatchModel> Patch) : ICommand<Result<Stack>>
{
    internal sealed class Validator : AbstractValidator<PatchStack>
    {
        public Validator()
        {
            RuleFor(x => x.Patch)
                .NotNull()
                .WithMessage("Patch cannot be null.");

            RuleFor(x => x.Patch.Patch)
                .Must(patch => patch.ValueKind == JsonValueKind.Object)
                .WithMessage("Patch must be a JSON object.")
                .When(x => x.Patch is not null);
        }
    }
}

internal sealed class PatchStackHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<PatchStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(PatchStack command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack == null || stack.CurrentStackRelease == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided stack does not exist."));
        }

        var current = new StackPatchModel(
            Name: stack.Name,
            PlatformId: stack.CurrentStackRelease.PlatformId,
            Description: stack.Description,
            StackSource: stack.StackSource,
            Spec: stack.CurrentStackRelease.Spec,
            DriftPolicy: stack.DriftPolicy);

        var patched = command.Patch.ApplyTo(current, StackJsonContext.Default.StackPatchModel);

        if (patched.DriftPolicy is { RemoveExtraContainers: true, Mode: not StackDriftMode.AutoFix })
        {
            return Result.Failure<Stack>(new BadRequestError("RemoveExtraContainers can only be enabled when drift mode is AutoFix."));
        }

        if (patched.PlatformId == null || patched.PlatformId == Guid.Empty)
        {
            return Result.Failure<Stack>(new BadRequestError("PlatformId is required."));
        }

        if (patched.Spec == null)
        {
            return Result.Failure<Stack>(new BadRequestError("Spec is required."));
        }

        var stackSource = patched.StackSource ?? stack.StackSource;
        if (stackSource != stack.StackSource)
        {
            return Result.Failure<Stack>(new BadRequestError("Changing StackSource is not supported."));
        }

        if (!IsCompatible(stackSource, patched.Spec))
        {
            return Result.Failure<Stack>(new BadRequestError("StackSource does not match the provided StackSpec."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(patched.PlatformId.Value, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided platform does not exist."));
        }

        if (patched.Spec is GitStack gitSpec)
        {
            var gitRepository = await unitOfWork.GitRepositories.GetAsync(gitSpec.GitRepoId, cancellationToken);
            if (gitRepository == null)
            {
                return Result.Failure<Stack>(new NotFoundError("The provided git repository does not exist."));
            }
        }

        var oldSnapshot = stack.ToSnapshot();

        if (PatchTouchesReleaseDefinition(command.Patch.Patch))
        {
            stack.UpdateCurrentStackReleaseDefinition(patched.PlatformId.Value, patched.Spec);
        }

        stack.UpdateDetails(driftPolicy: patched.DriftPolicy);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: stack.Id,
            resourceName: stack.Name,
            status: ActivityStatus.Success,
            platformId: stack.CurrentStackRelease.PlatformId,
            eventType: ActivityEventType.StackUpdated,
            info: new StackUpdated(oldSnapshot, stack.ToSnapshot()));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
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

    private static bool IsCompatible(StackSource stackSource, StackUpdateState stackUpdateState)
        => (stackSource, stackUpdateState) switch
        {
            (StackSource.WebEditor, ManualStackUpdateState) => true,
            (StackSource.Git, GitStackUpdateState) => true,
            _ => false
        };

    private static bool PatchTouchesReleaseDefinition(JsonElement patch)
    {
        if (patch.ValueKind != JsonValueKind.Object)
            return false;

        foreach (var property in patch.EnumerateObject())
        {
            if (property.NameEquals(nameof(StackPatchModel.PlatformId))
                || property.NameEquals("platformId")
                || property.NameEquals(nameof(StackPatchModel.Spec))
                || property.NameEquals("spec"))
            {
                return true;
            }
        }

        return false;
    }
}
