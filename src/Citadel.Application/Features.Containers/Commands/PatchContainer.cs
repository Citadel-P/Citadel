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

internal class PatchContainerHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetPlatformsByContainersId(request.ContainerIds, out var platformContainers))
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        foreach (var platform in platformContainers)
        {
            var command = new PatchContainerCommand
            (
                Action: request.Action,
                PlatformAddress: platform.PlatformAddress,
                ContainerIds: platform.Containers.Select(s => s.Key)
            );
            await connectorFactory.GetConnector(platform.Type).PatchAsync(command, cancellationToken);
        }

        return Result.Success();
    }
}