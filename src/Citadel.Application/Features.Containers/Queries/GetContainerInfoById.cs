using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

[RequirePermission(nameof(AppPermission.Container_View))]
public sealed record GetContainerInfoById(string ContainerId) : IQuery<Result<ContainerInfo>>
{
    internal class Validator : AbstractValidator<GetContainerById>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal class GetContainerInfoByIdHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainerInfoById, Result<ContainerInfo>>
{
    public async ValueTask<Result<ContainerInfo>> Handle(GetContainerInfoById query, CancellationToken cancellationToken)
    {
        var container = await unitOfWork.Containers.GetContainerInfoAsync(query.ContainerId, cancellationToken);
        return container ?? Result.Failure<ContainerInfo>(new NotFoundError("Container does not exist"));
    }
}
