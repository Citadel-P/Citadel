using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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

internal sealed class InspectVolumeHandler(IConnectorFactory<IVolumeConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<InspectVolume, Result<DockerVolume>>
{
    public async ValueTask<Result<DockerVolume>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<DockerVolume>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var command = new InspectVolumeCommand
        (
            Name: query.Name,
            PlatformAddress: platform.Address
        );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await volumeConnector.InspectVolumeAsync(command, cancellationToken);
    }
}