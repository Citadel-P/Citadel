using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record GetContainerById(string ContainerId) : IQuery<Result<Container>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator() 
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerByIdHandler(
    IUnitOfWork unitOfWork,
    IContainerAuthorizationService containerAuthorizationService) : IQueryHandler<GetContainerById, Result<Container>>
{
    public async ValueTask<Result<Container>> Handle(GetContainerById query, CancellationToken cancellationToken)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync([query.ContainerId], ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<Container>(new ForbiddenError("Missing permission [Read] on [Platform]"));
        }

        var container = await unitOfWork.Containers.GetContainerInfoAsync(query.ContainerId, cancellationToken);
        return container ?? Result.Failure<Container>(new NotFoundError("Container does not exist"));
    }
}
