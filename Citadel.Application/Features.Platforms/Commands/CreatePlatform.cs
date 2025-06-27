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
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(nameof(AppPermission.Platform_Create))]
public sealed record CreatePlatform(string Name, string Address, PlatformType Type, PlatformConnectorType ConnectorType) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<CreatePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            When(x => x.ConnectorType != PlatformConnectorType.Local, () => RuleFor(x => x.Address).ValidHostOrIp());
        }
    }
}

internal sealed class CreatePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IConnectorFactory<IPlatformConnector> platformConnectorFactory,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    ILogger<PatchPlatformHandler> logger) : ICommandHandler<CreatePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(CreatePlatform command, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await unitOfWork.Platforms.Query().AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        if (command.Type == PlatformType.Docker)
        {
            return await HandleDockerPlatform(command, cancellationToken);
        }
        else
        {
            return Result.Failure<Platform>(new BadRequestError("Only Docker platform type is supported at the moment."));
        }
    }

    private async Task<Result<Platform>> HandleDockerPlatform(CreatePlatform command, CancellationToken cancellationToken)
    {
        var param = new GetPlatformCommand
        (
            PlatformAddress: command.Address,
            PlatformName: command.Name
        );
        var platformConnector = platformConnectorFactory.GetConnector(command.ConnectorType);
        var response = await platformConnector.GetPlatformAsync(param, cancellationToken);
        if (!response.IsSuccess(out var platformResult, out var error))
        {
            return Result.Failure<Platform>(new InternalServerError($"Failed to get platform info for {command.Address}: {error?.Message}"));
        }

        // Add platform the new platform
        var platform = platformResult.Map(command.Address, command.Name, command.ConnectorType);
        unitOfWork.Platforms.Add(platform);
        await unitOfWork.SaveChangesAsync(cancellationToken);

        // Add containers
        var containers = await GetContainers(platform, cancellationToken);
        if (containers != null && containers.Any())
        {
            await unitOfWork.BulkInsertAsync(containers, cancellationToken: cancellationToken);
        }

        // Start tracking the platform
        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id, platform.ConnectorType);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<IEnumerable<Container>> GetContainers(Platform platform, CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand
            (
                PlatformId: platform.Id,
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

        return containers.Values.Map(platform.Id);
    }
}
