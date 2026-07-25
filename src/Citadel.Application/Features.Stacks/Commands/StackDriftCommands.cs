using Application.Services;
using Application.Services.Licensing;
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

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record UpdateStackDriftPolicy(Guid Id, StackDriftPolicy Policy) : ICommand<Result<Stack>>
{
    internal sealed class Validator : AbstractValidator<UpdateStackDriftPolicy>
    {
        public Validator()
        {
            RuleFor(x => x.Policy).NotNull();
            RuleFor(x => x.Policy).Must(BeSafePolicy)
                .WithMessage("RemoveExtraContainers can only be enabled when drift mode is AutoFix.");
        }

        private static bool BeSafePolicy(StackDriftPolicy policy)
            => !policy.RemoveExtraContainers || policy.Mode == StackDriftMode.AutoFix;
    }
}

internal sealed class UpdateStackDriftPolicyHandler(
    IUnitOfWork unitOfWork,
    ILicenseEntitlementService entitlementService) : ICommandHandler<UpdateStackDriftPolicy, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(UpdateStackDriftPolicy command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack is null)
        {
            return Result.Failure<Stack>(new NotFoundError("The provided stack does not exist."));
        }

        if (StackLicenseConfigurationPolicy.ExpandsDriftPolicy(
                stack.DriftPolicy,
                command.Policy))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<Stack>(entitlementError);
        }

        stack.UpdateDetails(driftPolicy: command.Policy);

        await unitOfWork.Stacks.UpdateAsync(stack, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return stack;
    }
}

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ReconcileStack(Guid Id) : ICommand<Result<StackReconciliationResult>>;

internal sealed class ReconcileStackHandler(
    IStackReconciler reconciler) : ICommandHandler<ReconcileStack, Result<StackReconciliationResult>>
{
    public async ValueTask<Result<StackReconciliationResult>> Handle(ReconcileStack command, CancellationToken cancellationToken)
    {
        try
        {
            return await reconciler.ReconcileAsync(command.Id, cancellationToken);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<StackReconciliationResult>(new BadRequestError(ex.Message));
        }
    }
}
