using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Stacks.Commands;

public sealed record PatchStack(Guid Id, JsonMergePatchDocument<StackPatchModel> Patch) : ICommand<Result<Stack>>
{
    internal sealed class Validator : PatchCommandValidator<PatchStack, StackPatchModel>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: StackJsonContext.Default.StackPatchModel,
                modelValidator: new StackPatchModelValidator())
        { }
    }

    internal sealed class StackPatchModelValidator : AbstractValidator<StackPatchModel>
    {
        public StackPatchModelValidator()
        {
        }
    }
}

internal sealed class PatchStackHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : ICommandHandler<PatchStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(PatchStack command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

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
            Spec: stack.CurrentStackRelease.Spec);

        var patched = command.Patch.ApplyTo(current, StackJsonContext.Default.StackPatchModel);

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

        var releaseChanged = patched.PlatformId.Value != stack.CurrentStackRelease.PlatformId
            || !Equals(patched.Spec, stack.CurrentStackRelease.Spec);

        if (releaseChanged)
        {
            var nextRelease = StackRelease.Create(
                stackId: stack.Id,
                platformId: patched.PlatformId.Value,
                spec: patched.Spec,
                createdByActorId: actorId,
                version: StackRelease.GetNextVersion(stack.CurrentStackRelease.Version));

            stack.SetCurrentStackRelease(nextRelease);
        }

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }

    private static bool IsCompatible(StackSource stackSource, StackSpec spec)
        => (stackSource, spec) switch
        {
            (StackSource.Manual, ManualStack) => true,
            (StackSource.Git, GitStack) => true,
            _ => false
        };

    private static bool IsCompatible(StackSource stackSource, StackUpdateState stackUpdateState)
        => (stackSource, stackUpdateState) switch
        {
            (StackSource.Manual, ManualStackUpdateState) => true,
            (StackSource.Git, GitStackUpdateState) => true,
            _ => false
        };
}