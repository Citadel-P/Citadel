using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;

namespace Application.Features.Platforms.Queries;

public sealed record GetPlatformInfo(Guid Id) : IQuery<Result<Platform>>
{
    internal class Validator : AbstractValidator<GetPlatformInfo>
    {
        public Validator()
        {
            RuleFor(s => s.Id).NotNull();
        }
    }
}

internal class GetPlatformInfoHandler(ApplicationDbContext dbContext) : IQueryHandler<GetPlatformInfo, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(GetPlatformInfo query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms
            .AsNoTracking()
            .Include(s => s.Stats.Where(x => x.Created > DateTimeOffset.UtcNow.AddHours(-1).ToUnixTimeSeconds())) // Get platform stats for the last hour
            .SingleOrDefaultAsync(s => s.Id == query.Id, cancellationToken);

        return platform is null
            ? Result.Failure<Platform>(new NotFoundError("The requested platform does not exist"))
            : Result.Success(platform);
    }
}