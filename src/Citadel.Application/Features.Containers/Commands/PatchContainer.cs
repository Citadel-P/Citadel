using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Containers.Commands;

public sealed record PatchContainer(string[] ContainerIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainer>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerIds).NotNull().NotEmpty();
            RuleForEach(s => s.ContainerIds).ValidContainerId();
        }
    }
}

internal sealed class PatchContainerHandler(
    IUserContextAccessor userContext,
    IUnitOfWork unitOfWork,
    IContainerProcessingService containerService,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IHostApplicationLifetime applicationLifetime,
    ILogger<PatchContainerHandler> logger)
    : ICommandHandler<PatchContainer, Result>
{
    private static readonly TimeSpan CompletionTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan RollbackTimeout = TimeSpan.FromSeconds(5);

    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken ct)
    {
        var hasAccess = await containerAuthorizationService.HasAccessAsync(request.ContainerIds, ResourceType.Platform, PermissionLevel.Write, SpecificPermission.None, ct);
        if (!hasAccess)
        {
            return Result.Failure(new ForbiddenError("Missing permission [Write] on [Platform]"));
        }

        var protectionError = await SystemContainerProtection.GetErrorAsync(unitOfWork, request.ContainerIds, ct);
        if (protectionError is not null)
        {
            return Result.Failure(new ConflictError(protectionError));
        }

        var actorId = userContext.Current.ActorId;
        if (!platformContainerCache.TryGetPlatformsWithContainers(request.ContainerIds, out var platforms))
        {
            return Result.Failure(new NotFoundError(
                "Platform resolution failed for container IDs. Platform may be disconnected."));
        }

        var containerIds = platforms.SelectMany(p => p.Containers.Values).ToArray();
        var resources = await containerService.MarkProcessingAsync(containerIds, actorId, ct);

        if (resources.HasConflict)
        {
            return Result.Failure(new ConflictError(
                "One or more containers, deployments, or stacks are already processing another operation."));
        }

        if (resources.Containers.Count == 0)
        {
            return Result.Failure(new NotFoundError(
                "No containers found for the provided ID(s)."));
        }

        using var completionCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        completionCancellation.CancelAfter(CompletionTimeout);
        var completionToken = completionCancellation.Token;
        try
        {
            await containerService.NotifyProcessingAsync(resources, completionToken);

            foreach (var platform in platforms)
            {
                var result = await PatchPlatformAsync(platform, request.Action, completionToken);
                if (result.IsFailure())
                {
                    await TryRollbackProcessingAsync(resources, actorId);
                    return result;
                }
            }

            await containerService.CompleteProcessingAsync(resources, platforms, actorId);
            return Result.Success();
        }
        catch
        {
            await TryRollbackProcessingAsync(resources, actorId);
            throw;
        }
    }

    private async Task TryRollbackProcessingAsync(ProcessedResources resources, Guid actorId)
    {
        using var rollbackCancellation = new CancellationTokenSource(RollbackTimeout);
        try
        {
            await containerService.RollbackProcessingAsync(
                resources,
                actorId,
                rollbackCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to roll back container command claims");
        }
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
