using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
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
        var dockerContainers = new List<DockerContainer>();
        foreach (var container in containers)
        {
            var dockerContainer = new DockerContainer(
                Name: container.Name,
                Image: container.Image?.Name ?? string.Empty,
                Id: container.DockerContainerId,
                ImageId: container.DockerImageId,
                State: container.State,
                ControlState: container.ControlState,
                Created: container.Created,
                Stack: container.DockerStack,
                ContainerStat: null,
                Ports: container.Ports,
                IsSystem: container.IsSystem,
                SystemRole: container.SystemRole,
                HasCitadelOwnershipLabels: container.HasCitadelOwnershipLabels);
            dockerContainers.Add(dockerContainer);
        }
        return Result.Success<IEnumerable<DockerContainer>>(dockerContainers);
    }
}
