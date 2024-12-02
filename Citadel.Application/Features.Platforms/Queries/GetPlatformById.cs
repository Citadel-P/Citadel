using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Gets platform by id
/// </summary>
/// <param name="Id">Id of the platform</param>
public sealed record GetPlatformById(Guid Id) : IQuery<Result<Platform>>
{
    internal class Validator : AbstractValidator<GetPlatformById>
    {
        public Validator()
            => RuleFor(s => s.Id).NotNull();        
    }
}

internal class GetPlatformByIdHandler(ApplicationDbContext dbContext) : IQueryHandler<GetPlatformById, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(GetPlatformById query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms
                                        .AsNoTracking()
                                        .AsSplitQuery()
                                        .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1)) // We only care about the last record)
                                        .Include(s => s.SystemInfo)
                                        .ThenInclude(s => s.SwarmInfo)
                                        .ThenInclude(s => s.RemoteManagers)
                                        .SingleOrDefaultAsync(s => s.Id == query.Id, cancellationToken);

        return platform ?? Result.Fail<Platform>(new NotFoundError("Platform does not exist"));
    }
}