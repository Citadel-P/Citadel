using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record InspectContainer(string ContainerId) : IQuery<Result<ContainerInspectionInfo>>
{
    internal sealed class Validator : AbstractValidator<InspectContainer>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal sealed class InspectContainerHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IContainerConnector> connectorFactory)
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetPlatformWithContainer(query.ContainerId, out var platform))
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError($"No platform found for container ID {query.ContainerId}"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platform.Address, 
            ContainerId: query.ContainerId
        );
        return await connectorFactory.GetConnector(platform.ConnectorType).InspectAsync(command, cancellationToken);
    }
}