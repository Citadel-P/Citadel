using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Application.Features.ResourceBindings;
using Domain.Entities.ResourceBindings;
using Hosting.Common.Pipelines.Interfaces;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write)]
public sealed record CreateSwarmService(
    string Name,
    Guid PlatformId,
    string? Description,
    SwarmServiceSpec Spec,
    IReadOnlyCollection<Guid>? TagIds = null,
    ActivitySourceResource? DuplicateSource = null) : ICommand<Result<SwarmService>>
{
    internal sealed class Validator : AbstractValidator<CreateSwarmService>
    {
        public Validator()
        {
            RuleFor(command => command.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(command => command.PlatformId).NotEmpty();
            RuleFor(command => command.Description).MaximumLength(600);
            RuleFor(command => command.Spec).NotNull();
        }
    }
}

internal sealed class CreateSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ISwarmServiceStreamManager streamManager,
    ILicenseEntitlementService entitlementService,
    IPermissionService permissionService) : ICommandHandler<CreateSwarmService, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(CreateSwarmService command, CancellationToken cancellationToken)
    {
        if (SwarmServiceLicenseConfigurationPolicy.ExpandsOperationalGuardrails(null, command.Spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<SwarmService>(entitlementError);
        }
        if (SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(null, command.Spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<SwarmService>(entitlementError);
        }

        var validation = await SwarmServiceValidation.ValidateAsync(
            command.Spec, command.PlatformId, unitOfWork, userContext, cancellationToken);
        if (validation.IsFailure(out var error))
            return Result.Failure<SwarmService>(error);

        if (await unitOfWork.SwarmServices.ExistsAsync(command.PlatformId, command.Name, cancellationToken))
            return Result.Failure<SwarmService>(new ConflictError("Name already exists on this platform."));

        var duplicateSourceResult = await GetValidDuplicateSourceAsync(command.DuplicateSource, cancellationToken);
        if (!duplicateSourceResult.IsSuccess(out var duplicateSource))
            return Result.Failure<SwarmService>(duplicateSourceResult.Errors);

        IReadOnlyList<ResourceBinding> duplicateBindings = [];
        if (duplicateSource is not null)
        {
            var bindingsResult = await ResourceBindingsFeatureHelpers.GetDuplicateEntriesAsync(
                unitOfWork,
                permissionService,
                userContext.Current,
                ResourceType.SwarmService,
                ResourceBindingScope.SwarmService,
                duplicateSource.ResourceId,
                cancellationToken);
            if (bindingsResult.IsFailure(out var bindingError, out var entries))
                return Result.Failure<SwarmService>(bindingError);

            duplicateBindings = entries ?? [];
        }

        var service = new SwarmService(
            command.Name,
            command.PlatformId,
            userContext.Current.ActorId,
            command.Spec,
            command.Description);

        if (await unitOfWork.SwarmServices.DockerNameExistsAsync(command.PlatformId, service.DockerName, cancellationToken))
            return Result.Failure<SwarmService>(new ConflictError("The generated Docker Service name already exists."));

        var inserted = await unitOfWork.SwarmServices.AddAsync(
            service,
            cancellationToken,
            command.TagIds,
            userContext.Current.ActorId);
        if (inserted == 0)
        {
            if (await unitOfWork.SwarmServices.ExistsAsync(command.PlatformId, command.Name, cancellationToken)
                || await unitOfWork.SwarmServices.DockerNameExistsAsync(
                    command.PlatformId, service.DockerName, cancellationToken))
                return Result.Failure<SwarmService>(new ConflictError("A Service with the same name already exists on this platform."));

            return Result.Failure<SwarmService>(new BadRequestError("One or more tags do not exist."));
        }

        await ResourceBindingsFeatureHelpers.CopyDuplicateEntriesAsync(
            unitOfWork,
            ResourceBindingScope.SwarmService,
            service.Id,
            duplicateBindings,
            cancellationToken);

        var eventType = duplicateSource is null
            ? ActivityEventType.SwarmServiceCreated
            : ActivityEventType.SwarmServiceDuplicated;
        ActivityEventInfo eventInfo = duplicateSource is null
            ? new SwarmServiceCreated(service.ToActivitySnapshot())
            : new SwarmServiceDuplicated(service.ToActivitySnapshot(), duplicateSource);
        await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
            service.PlatformId,
            service.Id,
            userContext.Current.ActorId,
            service.Name,
            eventType,
            ActivityStatus.Information,
            eventInfo), cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        await streamManager.SendSwarmServiceInfo(service, "create");
        return Result.Success(service);
    }

    private async Task<Result<ActivitySourceResource?>> GetValidDuplicateSourceAsync(
        ActivitySourceResource? source,
        CancellationToken cancellationToken)
    {
        if (source is null)
            return Result.Success<ActivitySourceResource?>(null);

        if (source.ResourceType != ActivityResourceType.SwarmService)
            return Result.Failure<ActivitySourceResource?>(
                new BadRequestError("Duplicate source must be a Swarm Service."));

        var sourceService = await unitOfWork.SwarmServices.GetAsync(source.ResourceId, cancellationToken);
        if (sourceService is null)
            return Result.Failure<ActivitySourceResource?>(
                new NotFoundError("Duplicate source Swarm Service does not exist."));

        var user = userContext.Current;
        if (!user.IsAdmin
            && user.ActorId != Constants.SystemId
            && !await unitOfWork.SwarmServices.CanAccessAsync(
                user.UserId,
                source.ResourceId,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken))
        {
            return Result.Failure<ActivitySourceResource?>(
                new ForbiddenError("Missing permission [Read] on duplicate source Swarm Service."));
        }

        return Result.Success<ActivitySourceResource?>(new ActivitySourceResource(
            ActivityResourceType.SwarmService,
            sourceService.Id,
            sourceService.Name));
    }
}
