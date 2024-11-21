using FluentValidation;
using Mediator;
using LightResults;
using Microsoft.EntityFrameworkCore;
using Application.Features.Platforms.Queries.Models;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Get all containers from remote agent
/// </summary>
public sealed record class GetContainers(GetContainersQuery ContainersQuery, Guid PlatformId)
    : IQuery<Result<IEnumerable<ContainerInfo>>>
{
    internal class Validator : AbstractValidator<GetContainers>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotNull();
        }
    }
}

internal class GetContainersHandler(ApplicationDbContext dbContext)
    : IQueryHandler<GetContainers, Result<IEnumerable<ContainerInfo>>>
{
    public async ValueTask<Result<IEnumerable<ContainerInfo>>> Handle(GetContainers query, CancellationToken cancellationToken)
    {
        var containers = await dbContext.ContainersInfo
                            .Where(s => s.PlatformId == query.PlatformId)
                            .Include(s => s.Stats.OrderByDescending(s => s.CreatedAtUtc).Take(1))
                            .ToListAsync(cancellationToken);
        
        return Result.Ok<IEnumerable<ContainerInfo>>(containers.OrderByDescending(s => s.Created));
    }
}