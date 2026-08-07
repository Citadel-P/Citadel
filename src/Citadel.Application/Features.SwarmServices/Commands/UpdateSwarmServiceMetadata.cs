using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Domain.Entities.Activities;
using Hosting.Common.Abstraction;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Application.Services.SignalR;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record RenameSwarmService(Guid Id, string Name) : ICommand<Result<SwarmService>>
{
    internal sealed class Validator : AbstractValidator<RenameSwarmService>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record UpdateSwarmServiceMetadata(Guid Id, string? Description) : ICommand<Result<SwarmService>>;

internal sealed class RenameSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ISwarmServiceStreamManager streamManager)
    : ICommandHandler<RenameSwarmService, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(RenameSwarmService command, CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(command.Id, cancellationToken);
        if (service is null
            || !await SwarmServiceValidation.CanAccessPlatformAsync(
                service.PlatformId, unitOfWork, userContext, cancellationToken))
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));
        if (service.ControlState == ResourceControlState.Processing)
            return Result.Failure<SwarmService>(new ConflictError("The Service changed or has an operation in progress."));
        if (await unitOfWork.SwarmServices.ExistsAsync(service.Id, service.PlatformId, command.Name, cancellationToken))
            return Result.Failure<SwarmService>(new ConflictError("Name already exists on this platform."));

        var oldName = service.Name;
        service.Rename(command.Name);
        var activity = new ActivityEvent(
            service.PlatformId,
            service.Id,
            userContext.Current.ActorId,
            service.Name,
            ActivityEventType.SwarmServiceRenamed,
            ActivityStatus.Information,
            new SwarmServiceRenamed(oldName, service.Name));
        var result = await PersistAsync(unitOfWork, service, activity, cancellationToken);
        if (result.IsSuccess(out var updated))
            await streamManager.SendSwarmServiceInfo(updated);
        return result;
    }

    internal static async Task<Result<SwarmService>> PersistAsync(
        IUnitOfWork unitOfWork,
        SwarmService service,
        ActivityEvent activity,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return Result.Failure<SwarmService>(new ConflictError("The Service was changed by another operation. Reload and try again."));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success(await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken) ?? service);
    }
}

internal sealed class UpdateSwarmServiceMetadataHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ISwarmServiceStreamManager streamManager)
    : ICommandHandler<UpdateSwarmServiceMetadata, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(UpdateSwarmServiceMetadata command, CancellationToken cancellationToken)
    {
        if (command.Description?.Length > 600)
            return Result.Failure<SwarmService>(new BadRequestError("Description cannot exceed 600 characters."));

        var service = await unitOfWork.SwarmServices.GetAsync(command.Id, cancellationToken);
        if (service is null
            || !await SwarmServiceValidation.CanAccessPlatformAsync(
                service.PlatformId, unitOfWork, userContext, cancellationToken))
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));
        if (service.ControlState == ResourceControlState.Processing)
            return Result.Failure<SwarmService>(new ConflictError("The Service changed or has an operation in progress."));

        var oldSnapshot = service.ToActivitySnapshot();
        service.UpdateDescription(command.Description);
        var activity = new ActivityEvent(
            service.PlatformId,
            service.Id,
            userContext.Current.ActorId,
            service.Name,
            ActivityEventType.SwarmServiceUpdated,
            ActivityStatus.Information,
            new SwarmServiceUpdated(oldSnapshot, service.ToActivitySnapshot()));
        var result = await RenameSwarmServiceHandler.PersistAsync(unitOfWork, service, activity, cancellationToken);
        if (result.IsSuccess(out var updated))
            await streamManager.SendSwarmServiceInfo(updated);
        return result;
    }
}
