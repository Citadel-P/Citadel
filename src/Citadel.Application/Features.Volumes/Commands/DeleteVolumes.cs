using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Domain.Entities.Platforms;

namespace Application.Features.Volumes.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record DeleteVolumes(Guid PlatformId, string[] Names, bool? Force = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteVolumes>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.Names).NotNull().NotEmpty();
            RuleForEach(s => s.Names).ValidNameIdentifier();
        }
    }
}

internal class DeleteVolumesHandler(
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IVolumeConnector> connectorFactory,
    IUnitOfWork unitOfWork) : ICommandHandler<DeleteVolumes, Result>
{
    public async ValueTask<Result> Handle(DeleteVolumes command, CancellationToken cancellationToken)
    {
        var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
        {
            return Result.Failure(new ConflictError(
                "Node-local Volume deletion requires an explicit Node target and is not available."));
        }

        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error))
        {
            return Result.Failure(error);
        }

        var args = new DeleteDockerVolumeCommand
        (
            PlatformAddress: platform.Address,
            Names: command.Names,
            Force: command.Force ?? false
        );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await volumeConnector.DeleteVolumeAsync(args, cancellationToken);
    }
}
