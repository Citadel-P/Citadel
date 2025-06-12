using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure;
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

internal sealed class InspectContainerHandler(IContainerConnector containerConnector, ApplicationDbContext dbContext) 
    : IQueryHandler<InspectContainer, Result<ContainerInspectionInfo>>
{
    
    public async ValueTask<Result<ContainerInspectionInfo>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Containers.GetPlatformAddress(query.ContainerId, cancellationToken);
        if (platformAddress == null)
        {
            return Result.Failure<ContainerInspectionInfo>(new NotFoundError($"Platform doesn't exist for container {query.ContainerId}"));
        }

        var command = new InspectContainerCommand(platformAddress, query.ContainerId);
        return await containerConnector.InspectAsync(command, cancellationToken);
    }
}