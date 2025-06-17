using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Platforms.Queries;

/// <summary>
/// Get all containers from remote agent
/// </summary>
public sealed record class GetContainers(Guid PlatformId) : IQuery<Result<IEnumerable<Container>>>
{
    internal class Validator : AbstractValidator<GetContainers>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotNull();
        }
    }
}

internal class GetContainersHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainers, Result<IEnumerable<Container>>>
{
    public async ValueTask<Result<IEnumerable<Container>>> Handle(GetContainers query, CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Containers
                .WithLastStat(query.PlatformId)
                .ToListAsync(cancellationToken);
        
        return Result.Success<IEnumerable<Container>>(containers);
    }
}