using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Commands;

[RequirePermission(ResourceType.Platform, ResourceAction.Delete)]
public sealed record DeleteVolumes(Guid PlatformId, string[] Names, bool? Force = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteVolumes>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Names).ValidNameIdentifier();
        }
    }
}

internal class DeleteVolumesHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IVolumeConnector> connectorFactory) : ICommandHandler<DeleteVolumes, Result>
{
    public async ValueTask<Result> Handle(DeleteVolumes command, CancellationToken cancellationToken)
    {
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