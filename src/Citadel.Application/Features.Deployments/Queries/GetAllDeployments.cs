using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Deployments.Queries
{
    public sealed record GetAllDeployments : IQuery<Result<IEnumerable<Deployment>>>;

    internal sealed class GetAllDeploymentsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllDeployments, Result<IEnumerable<Deployment>>>
    {
        public async ValueTask<Result<IEnumerable<Deployment>>> Handle(GetAllDeployments query, CancellationToken cancellationToken)
        {
            var user = userContextAccessor.Current;
            var deployments = user is not null && !user.IsAdmin
                ? await unitOfWork.Deployments.GetAuthorizedInfoAsync(user.UserId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
                : await unitOfWork.Deployments.GetInfoAsync(cancellationToken);

            return Result.Success(deployments);
        }
    }
}
