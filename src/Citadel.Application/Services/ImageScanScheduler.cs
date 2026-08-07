using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Registries;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal interface IImageScanScheduler
{
    Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<ManualStackImageCheck>> LoadManualStackChecksAsync(CancellationToken cancellationToken);
    Task<IReadOnlyCollection<SwarmServiceImageCheck>> LoadSwarmServiceChecksAsync(CancellationToken cancellationToken);
}

internal sealed class ImageScanScheduler(
    IServiceScopeFactory scopeFactory,
    IImageCheckBuilder? imageCheckBuilder = null) : IImageScanScheduler
{
    private readonly IImageCheckBuilder imageCheckBuilder = imageCheckBuilder ?? new ImageCheckBuilder();

    public async Task<IReadOnlyCollection<ImageScanTask>> LoadScanTasksAsync(CancellationToken cancellationToken)
    {
        var deploymentChecks = await LoadDeploymentChecksAsync(cancellationToken);
        var stackChecks = await LoadManualStackChecksAsync(cancellationToken);
        var serviceChecks = await LoadSwarmServiceChecksAsync(cancellationToken);
        var registriesById = await LoadRegistriesAsync(deploymentChecks, stackChecks, serviceChecks, cancellationToken);
        var scanTasks = new Dictionary<ImageKey, ImageScanTask>();

        foreach (var check in deploymentChecks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry))
                continue;

            var taskResult = imageCheckBuilder.BuildScanTask(check.Key, check.Deployment.PlatformId, registry);
            if (taskResult.IsSuccess(out var task))
                scanTasks.TryAdd(check.Key, task);
        }

        foreach (var check in stackChecks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry))
                continue;

            var taskResult = imageCheckBuilder.BuildScanTask(
                check.Key,
                check.Stack.CurrentStackRelease!.PlatformId,
                registry);
            if (taskResult.IsSuccess(out var task))
                scanTasks.TryAdd(check.Key, task);
        }

        foreach (var check in serviceChecks)
        {
            if (!registriesById.TryGetValue(check.Key.RegistryId, out var registry))
                continue;

            var taskResult = imageCheckBuilder.BuildScanTask(
                check.Key,
                check.Service.PlatformId,
                registry);
            if (taskResult.IsSuccess(out var task))
                scanTasks.TryAdd(check.Key, task);
        }

        return scanTasks.Values;
    }

    public async Task<IReadOnlyCollection<DeploymentImageCheck>> LoadDeploymentChecksAsync(CancellationToken cancellationToken)
    {
        var deployments = await LoadDeploymentsAsync(cancellationToken);
        var checks = new List<DeploymentImageCheck>(deployments.Count());

        foreach (var deployment in deployments)
        {
            if (deployment.Platform?.PlatformDescriptor.Type != PlatformType.Docker)
                continue;

            var result = imageCheckBuilder.BuildDeploymentCheck(deployment, ImageCheckMode.Scheduled);
            if (result.IsSuccess(out var check))
                checks.Add(check);
        }

        return checks;
    }

    public async Task<IReadOnlyCollection<ManualStackImageCheck>> LoadManualStackChecksAsync(CancellationToken cancellationToken)
    {
        var stacks = await LoadStacksAsync(cancellationToken);
        var checks = new List<ManualStackImageCheck>();

        foreach (var stack in stacks)
        {
            if (stack.CurrentStackRelease?.Platform?.PlatformDescriptor.Type != PlatformType.Docker)
                continue;

            var result = imageCheckBuilder.BuildManualStackChecks(stack, ImageCheckMode.Scheduled);
            if (result.IsSuccess(out var stackChecks))
                checks.AddRange(stackChecks);
        }

        return checks;
    }

    public async Task<IReadOnlyCollection<SwarmServiceImageCheck>> LoadSwarmServiceChecksAsync(
        CancellationToken cancellationToken)
    {
        var services = await LoadSwarmServicesAsync(cancellationToken);
        var checks = new List<SwarmServiceImageCheck>(services.Count);

        foreach (var service in services)
        {
            if (service.Platform?.PlatformDescriptor.Type != PlatformType.DockerSwarm)
                continue;

            var result = imageCheckBuilder.BuildSwarmServiceCheck(service, ImageCheckMode.Scheduled);
            if (result.IsSuccess(out var check))
                checks.Add(check);
        }

        return checks;
    }

    private async Task<IEnumerable<Deployment>> LoadDeploymentsAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Deployments.GetAllAsync(cancellationToken) ?? [];
    }

    private async Task<IEnumerable<Stack>> LoadStacksAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.Stacks.GetAllAsync(cancellationToken) ?? [];
    }

    private async Task<IReadOnlyList<SwarmService>> LoadSwarmServicesAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return await uow.SwarmServices.GetAllAsync(cancellationToken);
    }

    private async Task<IReadOnlyDictionary<Guid, Registry>> LoadRegistriesAsync(
        IReadOnlyCollection<DeploymentImageCheck> deploymentChecks,
        IReadOnlyCollection<ManualStackImageCheck> stackChecks,
        IReadOnlyCollection<SwarmServiceImageCheck> serviceChecks,
        CancellationToken cancellationToken)
    {
        var registryIds = deploymentChecks
            .Select(c => c.Key.RegistryId)
            .Concat(stackChecks.Select(c => c.Key.RegistryId))
            .Concat(serviceChecks.Select(c => c.Key.RegistryId))
            .Distinct()
            .ToArray();

        if (registryIds.Length == 0)
            return new Dictionary<Guid, Registry>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var registries = await uow.Registries.GetAllAsync(registryIds, cancellationToken) ?? [];
        return registries.ToDictionary(r => r.Id);
    }

}
