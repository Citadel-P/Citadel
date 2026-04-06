using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Platform, ResourceAction.Update)]
public sealed record PatchContainer(string[] ContainerIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainer>
    {
        public Validator()
            => RuleForEach(s => s.ContainerIds).ValidContainerId();
    }
}

internal sealed class PatchContainerHandler(
    IContainerProcessingService containerService,
    IPlatformContainerCache platformContainerCache,
    IHttpContextAccessor httpContextAccessor,
    IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken ct)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError(
                "Platform resolution failed for container IDs. Platform may be disconnected."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var containers = await containerService.MarkProcessingAsync(containerIds, actorId, ct);

        if (containers.Count == 0)
        {
            return Result.Failure(new NotFoundError(
                "No containers found for the provided ID(s)."));
        }

        await containerService.NotifyProcessingAsync(containers, ct);

        foreach (var platform in platforms)
        {
            var result = await PatchPlatformAsync(platform, request.Action, ct);
            if (result.IsFailure())
            {
                await containerService.RollbackProcessingAsync(containers, actorId, ct);
                return result;
            }
        }

        return Result.Success();
    }

    private Task<Result> PatchPlatformAsync(PlatformCacheEntry platform, ContainerAction action, CancellationToken ct)
    {
        var command = new PatchContainerCommand(
            Action: action,
            PlatformAddress: platform.Address,
            ContainerIds: platform.Containers.Keys);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return connector.PatchAsync(command, ct);
    }
}