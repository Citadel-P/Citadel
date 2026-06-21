using Application.Services.Alerts;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using System.Threading.Channels;

namespace Application.TaskJobs;

internal sealed class UnmanagedContainerAlertJob(
    ChannelReader<UnmanagedContainerAlertRequest> reader,
    IServiceScopeFactory scopeFactory,
    IAlertService alertService,
    ILogger<UnmanagedContainerAlertJob> logger) : BackgroundService
{
    private static readonly TimeSpan GracePeriod = TimeSpan.FromSeconds(30);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await foreach (var request in reader.ReadAllAsync(stoppingToken))
        {
            try
            {
                await Task.Delay(GracePeriod, stoppingToken);
                await ProcessAsync(request, stoppingToken);
            }
            catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to process unmanaged container alert for {DockerContainerId} on platform {PlatformId}", request.DockerContainerId, request.PlatformId);
            }
        }
    }

    private async Task ProcessAsync(UnmanagedContainerAlertRequest request, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var container = await uow.Containers.GetByIdAsync(request.DockerContainerId, cancellationToken);
        if (container is null || container.DeploymentId is not null || container.StackId is not null) 
            return;

        var platform = await uow.Platforms.GetByIdAsync(request.PlatformId, cancellationToken);
        if (platform is null) 
            return;

        var context = new AlertEvaluationContext(
            UtcNow: DateTime.UtcNow,
            Platforms: [],
            Deployments: [],
            Stacks: [],
            Containers:
            [
                new ContainerAlertSnapshot(
                    Id: container.Id,
                    PlatformId: container.PlatformId,
                    ContainerId: container.DockerContainerId,
                    Name: container.Name,
                    PlatformName: platform.Name,
                    PlatformAddress: platform.Address)
            ]);

        await alertService.ProcessAsync(AlertType.UnmanagedContainerCreated, context, cancellationToken);
    }
}

internal readonly record struct UnmanagedContainerAlertRequest(
    Guid PlatformId,
    string DockerContainerId);
