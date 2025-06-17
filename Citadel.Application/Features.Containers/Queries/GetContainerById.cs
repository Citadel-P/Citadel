using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Containers.Queries;

[RequirePermission(nameof(AppPermission.Container_View))]
public sealed record GetContainerById(string ContainerId) : IQuery<Result<Container>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator() 
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerByIdHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainerById, Result<Container>>
{
    public async ValueTask<Result<Container>> Handle(GetContainerById query, CancellationToken cancellationToken)
    {
        var container = await unitOfWork.Containers
            .Query().AsNoTracking()
            .Include(s => s.Platform)
            .FirstOrDefaultAsync(s => s.ContainerId.StartsWith(query.ContainerId), cancellationToken: cancellationToken);

        return container ?? Result.Failure<Container>(new NotFoundError("Platform does not exist"));
    }
}
