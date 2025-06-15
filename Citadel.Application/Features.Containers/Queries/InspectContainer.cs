using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
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

internal sealed class InspectContainerHandler(IConnectorFactory<IContainerConnector> connectorFactory, ApplicationDbContext dbContext)
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var (address, id, connectorType) = await dbContext.Containers.GetPlatformIdAsync(query.ContainerId, cancellationToken);
        if (string.IsNullOrEmpty(address) || id is null || connectorType is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError($"No platform found for container ID {query.ContainerId}"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: address, 
            ContainerId: query.ContainerId
        );
        return await connectorFactory.GetConnector(connectorType.Value).InspectAsync(command, cancellationToken);
    }
}