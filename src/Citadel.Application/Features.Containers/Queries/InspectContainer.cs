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

internal sealed class InspectContainerHandler(IUnitOfWork unitOfWork, IConnectorFactory<IContainerConnector> connectorFactory)
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var platformDetails = await unitOfWork.Platforms.GetPlatformDetailsByContainerIdAsync(query.ContainerId, cancellationToken);
        if (platformDetails is null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError($"No platform found for container ID {query.ContainerId}"));
        }

        var command = new InspectContainerCommand
        (
            PlatformAddress: platformDetails.Address, 
            ContainerId: query.ContainerId
        );
        return await connectorFactory.GetConnector(platformDetails.ConnectorType).InspectAsync(command, cancellationToken);
    }
}