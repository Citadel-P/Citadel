using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record PatchStackMetadata(Guid Id, JsonMergePatchDocument<StackPatchModel> Patch) : ICommand<Result<Stack>>
{
    internal sealed class Validator : PatchCommandValidator<PatchStackMetadata, StackPatchModel>
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
            When(x => x.Description != null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchStackMetadataHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchStackMetadata, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(PatchStackMetadata command, CancellationToken cancellationToken)
    {
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

        stack.UpdateDetails(description: patched.Description);

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }
}
