using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Application.Mappers;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetContainersData(Guid Id) : IQuery<Result<IEnumerable<DockerContainer>>>;

internal sealed class GetDockerContainersDataHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainersData, Result<IEnumerable<DockerContainer>>>
{
    public async ValueTask<Result<IEnumerable<DockerContainer>>> Handle(GetContainersData query, CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Stacks.GetContainersAsync(query.Id, cancellationToken);
        return Result.Success(containers.Select(static container => container.ToDockerContainer()));
    }
}
