using FluentValidation;
using Mediator;
using LightResults;
using Microsoft.EntityFrameworkCore;
using Application.Features.Platforms.Queries.Models;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Get all containers from remote agent
/// </summary>
public sealed record class GetContainers(GetContainersQuery ContainersQuery, Guid PlatformId)
    : IQuery<Result<IEnumerable<Container>>>
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
    : IQueryHandler<GetContainers, Result<IEnumerable<Container>>>
{
    public async ValueTask<Result<IEnumerable<Container>>> Handle(GetContainers query, CancellationToken cancellationToken)
    {
        var containers = await dbContext.Containers
                .WithLastStat(query.PlatformId)
                .ToListAsync(cancellationToken);
        
        return Result.Success<IEnumerable<Container>>(containers);
    }
}