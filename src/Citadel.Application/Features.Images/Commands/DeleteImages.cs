using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.Features.Images.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record DeleteImages(Guid PlatformId, string[] Ids, bool Force = false, bool NoPrune = false) : ICommand<Result<DeleteImageResult>>
{
    internal class Validator : AbstractValidator<DeleteImages>
    {
        public Validator()
        {
            RuleForEach(s => s.Ids).ValidHashId();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
        }
    }
}

internal sealed class DeleteImagesHandler(
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache, 
    IConnectorFactory<IImageConnector> connectorFactory,
    IHostApplicationLifetime applicationLifetime,
    ILogger<DeleteImagesHandler> logger
) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
{
    public async ValueTask<Result<DeleteImageResult>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var persistedPlatform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
            if (persistedPlatform?.PlatformDescriptor is DockerSwarmPlatformDescriptor)
            {
                return Result.Failure<DeleteImageResult>(new ConflictError(
                    "Node-local Image deletion requires an explicit Node target and is not available."));
            }
        }

        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error)) 
        {
            return Result.Failure<DeleteImageResult>(error);
        }

        var images = await MarkProcessingAsync(command.Ids, command.PlatformId, cancellationToken);
        if (images.Count == 0)
        {
            return Result.Failure<DeleteImageResult>(new NotFoundError("One or more images were not found or are already processing."));
        }
        await NotifyProcessingSafelyAsync(images, "update", cancellationToken);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var claimedIds = images.Select(static image => image.DockerImageId).ToArray();

        var args = new DeleteImageCommand
        (
            Ids: claimedIds,
            Force: command.Force,
            NoPrune: command.NoPrune,
            PlatformAddress: platform.Address
        );
        Result<DeleteImageResult> result;
        try
        {
            result = await connector.DeleteImageAsync(args, cancellationToken: cancellationToken);
        }
        catch
        {
            await TryReconcileAfterDeleteAsync(connector, platform.Address, images);
            throw;
        }

        if (result.IsSuccess())
        {
            using var completionCancellation = CreateCompletionCancellation();
            try
            {
                await DeletePersistedImagesAsync(images, completionCancellation.Token);
            }
            catch
            {
                await TryReconcileAfterDeleteAsync(connector, platform.Address, images);
                throw;
            }
            await NotifyProcessingSafelyAsync(images, "delete", completionCancellation.Token);
            return result;
        }

        // A multi-image daemon request can partially succeed before reporting an error.
        // Reconcile against the daemon rather than blindly releasing every claim.
        await TryReconcileAfterDeleteAsync(connector, platform.Address, images);
        return result;
    }

    private async Task DeletePersistedImagesAsync(IEnumerable<Image> images, CancellationToken ct)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.Images.DeleteAsync(images.Select(static image => image.Id), ct);
        await uow.CommitAsync(ct);
    }

    public async Task<List<Image>> MarkProcessingAsync(string[] ids, Guid platformId, CancellationToken ct)
    {
        var updated = new List<Image>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var requestedIds = ids.Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
        var images = (await uow.Images.GetByIdAsync(requestedIds, platformId, ct)).ToArray();
        if (requestedIds.Length == 0 || images.Length != requestedIds.Length)
            return [];

        foreach (var image in images)
        {
            if (!image.MarkProcessing())
                return [];

            var affected = await uow.Images.UpdateProcessingAsync(
                image.Id,
                image.ControlState,
                image.ControlStartedAt,
                image.RowVersion,
                checkRowVersion: true,
                ct);

            if (affected == 0)
                return [];

            updated.Add(image);
        }

        await uow.CommitAsync(ct);
        return updated;
    }

    public async Task<IEnumerable<Image>> RollbackProcessingAsync(IEnumerable<Image> images, CancellationToken ct)
    {
        var failedUpdates = new List<Image>();
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        foreach (var image in images)
        {
            image.ReleaseProcessing();

            var affected = await uow.Images.UpdateProcessingAsync(
                image.Id,
                image.ControlState,
                image.ControlStartedAt,
                image.RowVersion + 1,
                checkRowVersion: true,
                ct);

            if (affected != 0)
            {
                failedUpdates.Add(image);
            }
        }

        await uow.CommitAsync(ct);
        return failedUpdates;
    }

    private async Task ReconcileAfterDeleteAsync(
        IImageConnector connector,
        string platformAddress,
        IReadOnlyCollection<Image> claimedImages)
    {
        using var completionCancellation = CreateCompletionCancellation();
        var completionToken = completionCancellation.Token;
        var listed = await connector.ListImagesAsync(platformAddress, completionToken);
        if (!listed.IsSuccess(out var daemonImages))
        {
            var rolledBack = await RollbackProcessingAsync(claimedImages, completionToken);
            await NotifyProcessingSafelyAsync(rolledBack, "update", completionToken);
            return;
        }

        var daemonIds = daemonImages.Select(static image => NormalizeImageId(image.Id)).ToArray();
        var deleted = claimedImages
            .Where(image => !ContainsImageId(daemonIds, image.DockerImageId))
            .ToArray();
        var remaining = claimedImages
            .Where(image => ContainsImageId(daemonIds, image.DockerImageId))
            .ToArray();

        if (deleted.Length > 0)
        {
            await DeletePersistedImagesAsync(deleted, completionToken);
            await NotifyProcessingSafelyAsync(deleted, "delete", completionToken);
        }

        if (remaining.Length > 0)
        {
            var rolledBack = await RollbackProcessingAsync(remaining, completionToken);
            await NotifyProcessingSafelyAsync(rolledBack, "update", completionToken);
        }
    }

    private async Task TryReconcileAfterDeleteAsync(
        IImageConnector connector,
        string platformAddress,
        IReadOnlyCollection<Image> claimedImages)
    {
        try
        {
            await ReconcileAfterDeleteAsync(connector, platformAddress, claimedImages);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to reconcile image deletion after the daemon operation ended unexpectedly");
        }
    }

    private async Task NotifyProcessingSafelyAsync(
        IEnumerable<Image> images,
        string action,
        CancellationToken cancellationToken)
    {
        using var notificationCancellation = CancellationTokenSource.CreateLinkedTokenSource(
            cancellationToken,
            applicationLifetime.ApplicationStopping);
        notificationCancellation.CancelAfter(TimeSpan.FromSeconds(5));
        try
        {
            await NotifyProcessingAsync(images, action, notificationCancellation.Token);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to enqueue image {Action} notifications", action);
        }
    }

    private CancellationTokenSource CreateCompletionCancellation()
    {
        var cancellation = CancellationTokenSource.CreateLinkedTokenSource(
            applicationLifetime.ApplicationStopping);
        cancellation.CancelAfter(TimeSpan.FromSeconds(10));
        return cancellation;
    }

    private static bool ContainsImageId(IEnumerable<string> daemonIds, string requestedId)
    {
        var normalized = NormalizeImageId(requestedId);
        return daemonIds.Any(id =>
            string.Equals(id, normalized, StringComparison.OrdinalIgnoreCase)
            || (normalized.Length >= 12 && id.StartsWith(normalized, StringComparison.OrdinalIgnoreCase))
            || (id.Length >= 12 && normalized.StartsWith(id, StringComparison.OrdinalIgnoreCase)));
    }

    private static string NormalizeImageId(string id)
        => id.StartsWith("sha256:", StringComparison.OrdinalIgnoreCase) ? id[7..] : id;

    public async Task NotifyProcessingAsync(IEnumerable<Image> images, string action, CancellationToken ct)
    {
        foreach (var image in images)
        {
            await notificationQueue.EnqueueAsync(new ImageNotificationWorkItem(dockerDaemonHub, image, action), ct);
        }
    }
}
