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

internal class ListVolumesHandler(IVolumeConnector volumeConnector, ApplicationDbContext dbContext) : IQueryHandler<ListVolumes, Result<IEnumerable<DockerVolume>>>
{
    public async ValueTask<Result<IEnumerable<DockerVolume>>> Handle(ListVolumes query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<IEnumerable<DockerVolume>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new ListVolumesCommand
            (
                PlatformAddress: address,
                Dangling: query.Dangling,
                Driver: query.Driver,
                Name: query.Name
            );

        return await volumeConnector.ListVolumesAsync(args, cancellationToken);
    }
}