using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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

internal class GetPlatformByIdHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetPlatformById, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(GetPlatformById query, CancellationToken cancellationToken)
    {
        // Only include the most recent stat for the platform
        var platform = await unitOfWork.Platforms
                .Query().AsNoTracking()
                .Include(s => s.Stats.OrderByDescending(s => s.Created).Take(1))
                .SingleOrDefaultAsync(s => s.Id == query.Id, cancellationToken);

        return platform ?? Result.Failure<Platform>(new NotFoundError("Platform does not exist"));
    }
}