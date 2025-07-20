using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record PatchContainer(string[] ContainerIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainer>
    {
        public Validator()
            => RuleForEach(s => s.ContainerIds).ValidContainerId();
    }
}

internal class PatchContainerHandler(IUnitOfWork unitOfWork, IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken cancellationToken)
    {
        var platformContainers = await unitOfWork.Containers.GetPlatformsByContainerIdsAsync(request.ContainerIds, cancellationToken);

        if (platformContainers == null)
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        foreach (var platform in platformContainers)
        {
            var command = new PatchContainerCommand
            (
                PlatformAddress: platform.Address,
                ContainerIds: platform.ContainerIds,
                Action: request.Action
            );
            await connectorFactory.GetConnector(platform.ConnectorType).PatchAsync(command, cancellationToken);
        }

        return Result.Success();
    }
}