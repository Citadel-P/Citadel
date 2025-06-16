using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IReadOnlyList<DockerImage>>>;

internal class GetAllLocalImagesHandler(IConnectorFactory<IImageConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<GetAllLocalImages, Result<IReadOnlyList<DockerImage>>>
{
    public async ValueTask<Result<IReadOnlyList<DockerImage>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null) 
        {
            return Result.Failure<IReadOnlyList<DockerImage>>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        return await connectorFactory
            .GetConnector(platform.ConnectorType)
            .ListImagesAsync(platform.Address, cancellationToken: cancellationToken);
    }
}
