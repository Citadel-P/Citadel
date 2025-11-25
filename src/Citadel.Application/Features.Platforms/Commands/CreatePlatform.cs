using Application.Features.Images.Queries;
using Application.Mappers;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(nameof(AppPermission.Platform_Create))]
public sealed record CreatePlatform(string Name, string? Address, PlatformType Type, PlatformConnectorType ConnectorType) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<CreatePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            When(x => x.ConnectorType != PlatformConnectorType.Local, () => RuleFor(x => x.Address!).ValidHostOrIP());
        }
    }
}

internal sealed class CreatePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IConnectorFactory<IPlatformConnector> platformConnectorFactory,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    ILogger<PatchPlatformHandler> logger) : ICommandHandler<CreatePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(CreatePlatform command, CancellationToken cancellationToken)
    {
        if (command.ConnectorType == PlatformConnectorType.Local)
        {
            command = command with { Address = Constants.LocalDockerHostUrl };
        }
        // Check if the platform already exists
        if (await unitOfWork.Platforms.NameOrAddressExistsAsync(command.Name, command.Address!, cancellationToken: cancellationToken))
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same name or address already exists."));
        }

        if (command.Type == PlatformType.Docker)
        {
            return await HandleDockerPlatform(command, cancellationToken);
        }
        else
        {
            return Result.Failure<Platform>(new BadRequestError("Currently, only the Docker platform type is supported."));
        }
    }

    private async Task<Result<Platform>> HandleDockerPlatform(CreatePlatform command, CancellationToken cancellationToken)
    {
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

        // Add the new platform
        var platform = platformResult.Map(command.Address ?? "", command.Name, command.ConnectorType);
        await unitOfWork.Platforms.AddPlatformAsync(platform, cancellationToken);

        // Add it's images
        var images = await GetImages(platform, cancellationToken);
        if (images != null && images.Any())
        {
            await unitOfWork.Images.BulkUpsertAsync(images, cancellationToken);
        }

        // Add it's containers
        var containers = await GetContainers(images ?? [], platform, cancellationToken);
        if (containers != null && containers.Any())
        {
            await unitOfWork.Containers.BulkUpsertAsync(containers, cancellationToken);
        }
        // Commit
        await unitOfWork.CommitAsync(cancellationToken);

        // Start tracking the platform
        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id, platform.ConnectorType);

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

    private async Task<IEnumerable<Container>> GetContainers(IEnumerable<Image> images, Platform platform, CancellationToken cancellationToken)
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
            return [];
        }

        return [.. containers.Values.Map(images, platform.Id)];
    }
}
