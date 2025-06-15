using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Queries;

public sealed record ListVolumes(Guid PlatformId, bool? Dangling = null, string? Driver = null, string? Name = null) 
    : IQuery<Result<IEnumerable<DockerVolume>>>;

internal class ListVolumesHandler(IConnectorFactory<IVolumeConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolume>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolume>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<IEnumerable<DockerVolume>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListVolumesCommand
            (
                PlatformAddress: platform.Address,
                Dangling: query.Dangling,
                Driver: query.Driver,
                Name: query.Name
            );

        var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await volumeConnector.ListVolumesAsync(args, cancellationToken);
    }
}