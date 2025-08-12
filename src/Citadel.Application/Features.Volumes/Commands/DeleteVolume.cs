using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Commands;

public sealed record DeleteVolume(Guid PlatformId, string[] Names, bool? Force = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteVolume>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Names).ValidNameIdentifier();
        }
    }
}

internal class DeleteVolumeHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IVolumeConnector> connectorFactory) : ICommandHandler<DeleteVolume, Result>
{
    public async ValueTask<Result> Handle(DeleteVolume command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform))
        {
            return Result.Failure(new NotFoundError("Platform ID not found."));
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