using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Features.Images.Commands;

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
    IUnitOfWork unitOfWork,
    IServiceScopeFactory scopeFactory,
    INotificationQueue notificationQueue,
    IDockerDaemonStreamManager dockerDaemonHub,
    IPlatformContainerCache platformContainerCache, 
    IConnectorFactory<IImageConnector> connectorFactory 
) : ICommandHandler<DeleteImages, Result<DeleteImageResult>>
{
    public async ValueTask<Result<DeleteImageResult>> Handle(DeleteImages command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error)) 
        {
            return Result.Failure<DeleteImageResult>(error);
        }

        var images = await MarkProcessingAsync(command.Ids, command.PlatformId, cancellationToken);
        if (images.Count == 0)
        {
            return Result.Failure<DeleteImageResult>(new NotFoundError("No mages found for the provided ID(s)."));
        }
        await NotifyProcessingAsync(images, "update", cancellationToken);

        var args = new DeleteImageCommand
        (
            Ids: command.Ids,
            Force: command.Force,
            NoPrune: command.NoPrune,
            PlatformAddress: platform.Address
        );
        var result = await connectorFactory
            .GetConnector(platform.ConnectorType)
            .DeleteImageAsync(args, cancellationToken: cancellationToken);

        var failedUpdates = new List<Image>();
        // Handle the case where the image is not on the platform but still in db
        if (result.IsFailure(out var errorResult) && errorResult is NotFoundError)
        {
            // Todo: bulk delete
            foreach (var id in command.Ids)
            {
                var existing = await unitOfWork.Images.GetByDockerImageIdAsync(id, command.PlatformId, cancellationToken);
                if (existing == null) continue;
                failedUpdates.Add(existing);
                await unitOfWork.Images.DeleteAsync([existing.Id], cancellationToken);
            }
            await unitOfWork.CommitAsync(cancellationToken);

            await NotifyProcessingAsync(failedUpdates, "delete", cancellationToken);
        }

        return result;
    }

    public async Task<List<Image>> MarkProcessingAsync(string[] ids, Guid platformId, CancellationToken ct)
    {
        var updated = new List<Image>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var images = await uow.Images.GetByIdAsync(ids, platformId, ct);

        foreach (var image in images)
        {
            image.MarkProcessing();

            var affected = await uow.Images.UpdateProcessingAsync(
                image.Id,
                image.ControlState,
                image.ControlStartedAt,
                image.RowVersion,
                checkRowVersion: true,
                ct);

            if (affected != 0)
            {
                updated.Add(image);
            }
        }

        await uow.CommitAsync(ct);
        return updated;
    }

    public async Task NotifyProcessingAsync(IEnumerable<Image> images, string action, CancellationToken ct)
    {
        foreach (var image in images)
        {
            await notificationQueue.EnqueueAsync(new ImageNotificationWorkItem(dockerDaemonHub, image, action), ct);
        }
    }
}
