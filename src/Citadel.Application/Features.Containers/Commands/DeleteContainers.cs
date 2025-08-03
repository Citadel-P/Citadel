using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record DeleteContainers(string[] ContainerIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainerIds).ValidContainerId();
    }
}

internal sealed class DeleteContainersHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetPlatformsByContainersId(request.ContainerIds, out var platformContainers))
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        foreach (var platform in platformContainers)
        {
            var command = new DeleteContainerCommand
            (
                ContainerIds: platform.Containers.Select(s => s.Key),
                PlatformAddress: platform.Address,
                Volume: request.V,
                Force: request.Force,
                Link: request.Link
            );
            await connectorFactory.GetConnector(platform.ConnectorType).DeleteAsync(command, cancellationToken);
        }

        return Result.Success();
    }
}