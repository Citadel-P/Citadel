using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Containers.Queries;

[RequirePermission(nameof(AppPermission.ListContainers))]
public sealed record GetContainerById(string ContainerId) : IQuery<Result<ContainerInfo>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator() 
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerByIdHandler(ApplicationDbContext dbContext)
    : IQueryHandler<GetContainerById, Result<ContainerInfo>>
{
    public async ValueTask<Result<ContainerInfo>> Handle(GetContainerById query, CancellationToken cancellationToken)
    {
        var container = await dbContext.ContainersInfo.AsNoTracking()
                                .Include(s => s.Platform)
                                .FirstOrDefaultAsync(s => s.ContainerId.StartsWith(query.ContainerId), cancellationToken: cancellationToken);

        return container ?? Result.Failure<ContainerInfo>(new NotFoundError("Platform does not exist"));
    }
}
