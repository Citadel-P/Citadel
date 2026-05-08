using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetAllLocalImages(Guid PlatformId): IQuery<Result<IEnumerable<Image>>>;

internal class GetAllLocalImagesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllLocalImages, Result<IEnumerable<Image>>>
{
    public async ValueTask<Result<IEnumerable<Image>>> Handle(GetAllLocalImages query, CancellationToken cancellationToken)
    {
        var result = await unitOfWork.Images.GetByPlatformIdAsync(query.PlatformId, cancellationToken);

        return Result.Success(result);
    }
}
