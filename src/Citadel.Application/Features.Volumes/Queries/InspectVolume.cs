using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Volumes.Queries;

public sealed record InspectVolume (Guid PlatformId, string Name) : IQuery<Result<DockerVolumeResult>>
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

internal sealed class InspectVolumeHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IVolumeConnector> connectorFactory) : IQueryHandler<InspectVolume, Result<DockerVolumeResult>>
{
    public async ValueTask<Result<DockerVolumeResult>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform))
        {
            return Result.Failure<DockerVolumeResult>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var command = new InspectDockerVolumeCommand
        (
            Name: query.Name,
            PlatformAddress: platform.Address
        );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await volumeConnector.InspectVolumeAsync(command, cancellationToken);
    }
}