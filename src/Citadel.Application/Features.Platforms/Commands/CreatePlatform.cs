using System.Collections.Immutable;
using Application.Features.Images.Queries;
using Application.Features.Deployments.Notifications;
using Application.Features.Platforms;
using Application.Mappers;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record CreatePlatform(
    string Name,
    string? Address,
    string? Description,
    PlatformType Type,
    PlatformConnectorType ConnectorType,
    IReadOnlyCollection<Guid>? TagIds = null) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<CreatePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            When(x => x.Description is not null, () => RuleFor(x => x.Description).MaximumLength(600));
            When(x => x.ConnectorType == PlatformConnectorType.Agent, () => RuleFor(x => x.Address!).ValidHostOrIP());
        }
    }
}

internal sealed class CreatePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IConnectorFactory<IPlatformConnector> platformConnectorFactory,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IPlatformContainerCache platformContainerCache,
    IUserContextAccessor userContext,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    ILicenseQuotaService licenseQuotaService,
    ILogger<PatchPlatformHandler> logger) : ICommandHandler<CreatePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(CreatePlatform command, CancellationToken cancellationToken)
    {
        if (command.ConnectorType == PlatformConnectorType.Local)
        {
            command = command with { Address = Constants.LocalDockerHostUrl };
        }

        if (command.Type == PlatformType.Docker)
        {
            if (command.ConnectorType == PlatformConnectorType.EdgeAgent)
            {
                return await HandleEdgeDockerPlatform(command, cancellationToken);
            }

            if (await unitOfWork.Platforms.NameOrAddressExistsAsync(command.Name, command.Address!, cancellationToken: cancellationToken))
            {
                return Result.Failure<Platform>(new ConflictError("A platform with the same name or address already exists."));
            }

            return await HandleDockerPlatform(command, cancellationToken);
        }
        else
        {
            return Result.Failure<Platform>(new BadRequestError("Currently, only the Docker platform type is supported."));
        }
    }

    private async Task<Result<Platform>> HandleEdgeDockerPlatform(CreatePlatform command, CancellationToken cancellationToken)
    {
        var platformId = Guid.CreateVersion7();
        var address = $"edge://{platformId:D}";

        if (await unitOfWork.Platforms.NameOrAddressExistsAsync(command.Name, address, cancellationToken: cancellationToken))
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same name or address already exists."));
        }

        var quotaResult = await licenseQuotaService.EnsureCanIncreaseAsync(
            new Dictionary<LicenseLimit, int>
            {
                [LicenseLimit.Platforms] = 1,
                [LicenseLimit.EdgeAgentPlatforms] = 1
            },
            unitOfWork,
            cancellationToken);
        if (quotaResult.IsFailure())
            return Result.Failure<Platform>(quotaResult.Errors);

        var platform = Platform.FromPersistence(
            id: platformId,
            name: command.Name,
            address: address,
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 0,
            memTotal: 0,
            status: PlatformStatus.Offline,
            connectorType: PlatformConnectorType.EdgeAgent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: string.Empty,
                ContainerCount: 0,
                ContainersRunning: 0,
                ContainersPaused: 0,
                ContainersStopped: 0),
            serverVersion: null,
            agentVersion: null,
            description: command.Description);

        var actorId = userContext.Current.ActorId;
        var result = await unitOfWork.Platforms.AddAsync(platform, cancellationToken, command.TagIds, actorId);
        if (result == 0)
            return Result.Failure<Platform>(new BadRequestError("One or more tags do not exist."));

        var activity = PlatformActivity.Created(platform, actorId);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id, platform.ConnectorType);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        logger.LogInformation("A new edge platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<Result<Platform>> HandleDockerPlatform(CreatePlatform command, CancellationToken cancellationToken)
    {
        var quotaResult = await licenseQuotaService.EnsureCanIncreaseAsync(
            new Dictionary<LicenseLimit, int>
            {
                [LicenseLimit.Platforms] = 1
            },
            unitOfWork,
            cancellationToken);
        if (quotaResult.IsFailure())
            return Result.Failure<Platform>(quotaResult.Errors);

        var param = new GetPlatformCommand
        (
            PlatformAddress: command.Address ?? "",
            PlatformName: command.Name
        );
        var platformConnector = platformConnectorFactory.GetConnector(command.ConnectorType);
        var response = await platformConnector.GetPlatformAsync(param, cancellationToken);
        if (!response.IsSuccess(out var platformResult, out var error))
        {
            return Result.Failure<Platform>(new InternalServerError($"Failed to get platform info for {command.Address}: {error?.Message}"));
        }

        var platform = platformResult.Map(command.Address ?? "", command.Name, command.ConnectorType);
        platform.PartialUpdate(description: command.Description);
        var actorId = userContext.Current.ActorId;
        var result = await unitOfWork.Platforms.AddAsync(platform, cancellationToken, command.TagIds, actorId);
        if (result == 0)
            return Result.Failure<Platform>(new BadRequestError("One or more tags do not exist."));

        var images = (await GetImages(platform, cancellationToken)).ToArray();
        if (images.Length > 0)
        {
            await unitOfWork.Images.BulkUpsertAsync(images, cancellationToken);
        }

        var containers = await GetContainers(images, platform, cancellationToken);
        if (containers is not null)
        {
            ApplyDockerContainerCounts(platform, containers);
            await unitOfWork.Platforms.UpdateAsync(platform, cancellationToken);

            if (containers.Count > 0)
            {
                await unitOfWork.Containers.BulkUpsertAsync(containers, cancellationToken);
            }
        }

        var activity = PlatformActivity.Created(platform, actorId);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        PopulatePlatformCache(platform, containers);
        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id, platform.ConnectorType);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<IEnumerable<Image>> GetImages(Platform platform, CancellationToken cancellationToken)
    {
        var imageConnector = imageConnectorFactory.GetConnector(platform.ConnectorType);
        var imagesResult = await imageConnector.ListImagesAsync(platform.Address, cancellationToken: cancellationToken);
        if (!imagesResult.IsSuccess(out var images))
        {
            imagesResult.IsFailure(out var error);
            logger.LogError("Failed to list images for platform {Address}: {Error}", platform.Address, error?.Message);
            return [];
        }

        return [.. images.Map(platform.Id)];
    }

    private async Task<IReadOnlyCollection<Container>?> GetContainers(IEnumerable<Image> images, Platform platform, CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand
            (
                PlatformAddress: platform.Address,
                All: true
            );

        var containerConnector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var containersResult = await containerConnector.ListContainersAsync(command, cancellationToken: cancellationToken);

        if (!containersResult.IsSuccess(out var containers))
        {
            containersResult.IsFailure(out var error);
            logger.LogError("Failed to list containers for platform {Address}: {Error}", platform.Address, error?.Message);
            return null;
        }

        return [.. containers.Values.Map(images, platform.Id)];
    }

    private static void ApplyDockerContainerCounts(Platform platform, IReadOnlyCollection<Container> containers)
    {
        if (platform.PlatformDescriptor is not DockerPlatformDescriptor descriptor)
        {
            return;
        }

        var running = containers.LongCount(c => c.State is ContainerStateStatus.Running or ContainerStateStatus.Restarting);
        var paused = containers.LongCount(c => c.State == ContainerStateStatus.Paused);
        var stopped = containers.LongCount() - running - paused;

        platform.PartialUpdate(descriptor: descriptor.Create(
            containerCount: containers.Count,
            containersRunning: running,
            containersPaused: paused,
            containersStopped: stopped));
    }

    private void PopulatePlatformCache(Platform platform, IReadOnlyCollection<Container>? containers)
    {
        var containerMap = (containers ?? Array.Empty<Container>())
            .ToImmutableDictionary(
                container => container.DockerContainerId,
                container => container.Id,
                StringComparer.OrdinalIgnoreCase);

        platformContainerCache.ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                containerMap));
    }
}
