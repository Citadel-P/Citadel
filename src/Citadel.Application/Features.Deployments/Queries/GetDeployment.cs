using Domain.Contracts.Interfaces;
using Domain.Entities;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

public sealed record GetDeployment(Guid Id) : IQuery<Result<Deployment>>;

internal sealed class GetDeploymentHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(GetDeployment query, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken);
        return deployment ?? Result.Failure<Deployment>(new NotFoundError($"Deployment with ID {query.Id} does not exist"));
    }
}