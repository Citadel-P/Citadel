using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record CheckSwarmServiceUpdates(Guid Id) : ICommand<Result<SwarmService>>;

internal sealed class CheckSwarmServiceUpdatesHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ISwarmServiceUpdateCheckService updateCheckService)
    : ICommandHandler<CheckSwarmServiceUpdates, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(
        CheckSwarmServiceUpdates command,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(command.Id, cancellationToken);
        if (service is null
            || (!userContext.Current.IsAdmin
                && userContext.Current.ActorId != Constants.SystemId
                && !await unitOfWork.Platforms.CanAccessAsync(
                    userContext.Current.UserId, service.PlatformId, cancellationToken)))
            return Result.Failure<SwarmService>(new NotFoundError(
                "The managed Swarm Service does not exist."));

        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;
        var checkedService = await updateCheckService.CheckAsync(
            command.Id, actorId, applyWhenAvailable: false, cancellationToken);
        return checkedService.IsSuccess(out var result, out var error)
            ? Result.Success(result!.Service)
            : Result.Failure<SwarmService>(error!);
    }
}
