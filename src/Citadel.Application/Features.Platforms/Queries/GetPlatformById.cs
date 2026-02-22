using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Gets platform by id
/// </summary>
/// <param name="Id">The ID of the platform</param>
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
        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(query.Id, cancellationToken);
        return platform ?? Result.Failure<Platform>(new NotFoundError("Platform does not exist"));
    }
}