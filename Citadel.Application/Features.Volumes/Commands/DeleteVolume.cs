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

internal class DeleteVolumeHandler(IUnitOfWork unitOfWork, IConnectorFactory<IVolumeConnector> connectorFactory) : ICommandHandler<DeleteVolume, Result>
{
    public async ValueTask<Result> Handle(DeleteVolume command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(command.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteDockerVolumeCommand
        (
            PlatformAddress: platform.Value.Address,
            Names: command.Names,
            Force: command.Force ?? false
        );

        var volumeConnector = connectorFactory.GetConnector(platform.Value.ConnectorType);
        return await volumeConnector.DeleteVolumeAsync(args, cancellationToken);
    }
}