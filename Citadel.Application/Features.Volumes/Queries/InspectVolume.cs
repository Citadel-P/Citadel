using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

public sealed record InspectVolume (Guid PlatformId, string Name) : IQuery<Result<DockerVolume>>
{
    internal class Validator : AbstractValidator<InspectVolume>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.Name).NotNull();
        }
    }
}

internal sealed class InspectVolumeHandler(IUnitOfWork unitOfWork, IConnectorFactory<IVolumeConnector> connectorFactory) : IQueryHandler<InspectVolume, Result<DockerVolume>>
{
    public async ValueTask<Result<DockerVolume>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure<DockerVolume>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var command = new InspectVolumeCommand
        (
            Name: query.Name,
            PlatformAddress: address
        );

        var volumeConnector = connectorFactory.GetConnector(connectorType);
        return await volumeConnector.InspectVolumeAsync(command, cancellationToken);
    }
}