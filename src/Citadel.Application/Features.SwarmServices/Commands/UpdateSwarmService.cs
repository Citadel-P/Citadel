using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Application.Services.SignalR;
using Application.Services.Licensing;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record UpdateSwarmService(Guid Id, SwarmServiceSpec Spec, long RowVersion) : ICommand<Result<SwarmService>>
{
    internal sealed class Validator : AbstractValidator<UpdateSwarmService>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.Spec).NotNull();
            RuleFor(command => command.RowVersion).GreaterThanOrEqualTo(0);
        }
    }
}

internal sealed class UpdateSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ISwarmServiceStreamManager streamManager,
    ILicenseEntitlementService entitlementService) : ICommandHandler<UpdateSwarmService, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(UpdateSwarmService command, CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(command.Id, cancellationToken);
        if (service is null
            || !await SwarmServiceValidation.CanAccessPlatformAsync(
                service.PlatformId, unitOfWork, userContext, cancellationToken))
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));
        if (service.RowVersion != command.RowVersion)
            return Result.Failure<SwarmService>(new ConflictError("The Service was changed by another operation. Reload and try again."));
        if (service.ControlState == ResourceControlState.Processing)
            return Result.Failure<SwarmService>(new ConflictError("The Service cannot be edited while an operation is running."));

        var validation = await SwarmServiceValidation.ValidateAsync(
            command.Spec, service.PlatformId, unitOfWork, userContext, cancellationToken);
        if (validation.IsFailure(out var error))
            return Result.Failure<SwarmService>(error);
        if (SwarmServiceLicenseConfigurationPolicy.ExpandsOperationalGuardrails(service.Spec, command.Spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<SwarmService>(entitlementError);
        }
        if (SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(service.Spec, command.Spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<SwarmService>(entitlementError);
        }
        var oldSnapshot = service.ToActivitySnapshot();
        if (!service.UpdateSpec(command.Spec))
            return Result.Failure<SwarmService>(new ConflictError("Scheduling mode cannot be changed after the Service has been applied."));

        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return Result.Failure<SwarmService>(new ConflictError("The Service was changed by another operation. Reload and try again."));

        await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
            service.PlatformId,
            service.Id,
            userContext.Current.ActorId,
            service.Name,
            ActivityEventType.SwarmServiceUpdated,
            ActivityStatus.Information,
            new SwarmServiceUpdated(oldSnapshot, service.ToActivitySnapshot())), cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        service = await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken) ?? service;
        await streamManager.SendSwarmServiceInfo(service);
        return Result.Success(service);
    }
}
