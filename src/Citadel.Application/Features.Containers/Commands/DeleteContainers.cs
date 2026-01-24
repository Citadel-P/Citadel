using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
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
        {
            RuleFor(s => s.ContainerIds).NotEmpty().WithMessage("At least one container ID must be provided.");
            RuleForEach(s => s.ContainerIds).ValidContainerId();
        }
    }
}

internal sealed class DeleteContainersHandler(
    IContainerProcessingService containerService,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken ct)
    {
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var containers = await containerService.MarkProcessingAsync(containerIds, ct);

        if (containers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the provided ID(s)."));
        }

        await containerService.NotifyProcessingAsync(containers, ct);

        foreach (var platform in platforms)
        {
            var result = await DeleteFromPlatformAsync(platform, request, ct);
            if (result.IsFailure())
            {
                await containerService.RollbackProcessingAsync(containers, ct);
                return result;
            }
        }

        return Result.Success();
    }

    private async Task<Result> DeleteFromPlatformAsync(PlatformCacheEntry platform, DeleteContainers request, CancellationToken ct)
    {
        var command = new DeleteContainerCommand(
            ContainerIds: platform.Containers.Keys,
            PlatformAddress: platform.Address,
            Volume: request.V,
            Force: request.Force,
            Link: request.Link);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        await connector.DeleteAsync(command, ct);
        return Result.Success();
    }
}