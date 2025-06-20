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
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(command.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteDockerVolumeCommand
        (
            PlatformAddress: address,
            Names: command.Names,
            Force: command.Force ?? false
        );

        var volumeConnector = connectorFactory.GetConnector(connectorType);
        return await volumeConnector.DeleteVolumeAsync(args, cancellationToken);
    }
}