using Domain.Contracts.Interfaces;
using Domain.Entities;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries
{
    public sealed record GetAllDeployments : IQuery<Result<IEnumerable<Deployment>>>;

    internal sealed class GetAllDeploymentsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllDeployments, Result<IEnumerable<Deployment>>>
    {
        public async ValueTask<Result<IEnumerable<Deployment>>> Handle(GetAllDeployments query, CancellationToken cancellationToken)
        {
            var deployments = await unitOfWork.Deployments.GetInfoAsync(cancellationToken) ?? [];
            return Result.Success(deployments);
        }
    }
}
