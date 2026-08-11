using Application.Configs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmServiceStats(Guid PlatformId, string ServiceId, int Hours = 24)
    : IQuery<Result<SwarmServiceStatsResult>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmServiceStats>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.ServiceId).NotEmpty().MaximumLength(255);
            RuleFor(query => query.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
        }
    }
}

internal sealed class GetSwarmServiceStatsHandler(
    IUnitOfWork unitOfWork,
    IOptions<JobConfiguration> options)
    : IQueryHandler<GetSwarmServiceStats, Result<SwarmServiceStatsResult>>
{
    private const int MaximumTasks = 500;

    public async ValueTask<Result<SwarmServiceStatsResult>> Handle(
        GetSwarmServiceStats query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmServiceStatsResult>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<SwarmServiceStatsResult>(new BadRequestError("Service statistics are available only for Docker Swarm platforms."));

        var service = await unitOfWork.Swarm.GetServiceAsync(
            query.PlatformId,
            query.ServiceId,
            cancellationToken);
        if (service is null)
            return Result.Failure<SwarmServiceStatsResult>(new NotFoundError("Swarm service does not exist."));

        var tasks = await unitOfWork.Swarm.GetTasksAsync(
            query.PlatformId,
            MaximumTasks,
            cancellationToken,
            query.ServiceId);
        var currentTasks = SelectCurrentRunningTasks(tasks);
        var missingNodeIds = new HashSet<string>(StringComparer.Ordinal);
        // Stats are streamed at MonitoringInterval but persisted in batches at FlashInterval.
        // A persisted sample can legitimately be older than the stream cadence immediately
        // before the next flush, so coverage must allow for both intervals.
        var freshnessSeconds = Math.Max(
            30,
            options.Value.FlashInterval + options.Value.MonitoringInterval * 3);
        var freshAfter = DateTimeOffset.UtcNow.AddSeconds(-freshnessSeconds).ToUnixTimeSeconds();
        var containersByRuntimeIdentity = (await unitOfWork.Containers.GetByPlatformIdAsync(
                query.PlatformId,
                cancellationToken))
            .Where(static container => !string.IsNullOrWhiteSpace(container.DockerNodeId))
            .ToDictionary(
                static container => (container.DockerNodeId!, container.DockerContainerId),
                static container => container,
                RuntimeIdentityComparer.Instance);
        var candidateContainers = new List<Container>(currentTasks.Count);

        foreach (var task in currentTasks)
        {
            if (string.IsNullOrWhiteSpace(task.DockerContainerId))
            {
                missingNodeIds.Add(task.DockerNodeId);
                continue;
            }

            if (!containersByRuntimeIdentity.TryGetValue(
                    (task.DockerNodeId, task.DockerContainerId),
                    out var container)
                || container.ProjectionStaleSince is not null)
            {
                missingNodeIds.Add(task.DockerNodeId);
                continue;
            }

            candidateContainers.Add(container);
        }

        var statsByContainer = (await unitOfWork.ContainerStats.GetStatsAggregatedAsync(
                candidateContainers.Select(static container => container.Id).ToArray(),
                query.Hours,
                cancellationToken))
            .GroupBy(static stat => stat.ContainerId)
            .ToDictionary(static group => group.Key, static group => group.ToArray());
        var observedContainerIds = new List<Guid>(candidateContainers.Count);
        var sampleTimes = new List<long>(candidateContainers.Count);
        foreach (var container in candidateContainers)
        {
            var history = statsByContainer.GetValueOrDefault(container.Id) ?? [];
            var latest = history.LastOrDefault(static stat => stat.Created is not null);
            if (latest.Created is null || latest.Created < freshAfter)
            {
                missingNodeIds.Add(container.DockerNodeId!);
                continue;
            }

            observedContainerIds.Add(container.Id);
            sampleTimes.Add(latest.Created.Value);
        }

        var expectedTasks = Math.Max(0, service.DesiredTaskCount);
        var observedTasks = observedContainerIds.Count;
        var serviceStats = await unitOfWork.SwarmServiceStats.GetStatsAggregatedAsync(
            new SwarmServiceStatIdentity(
                service.PlatformId,
                service.DockerServiceId,
                service.SwarmServiceId,
                service.StackId,
                service.Name),
            query.Hours,
            cancellationToken);
        var aggregate = serviceStats
            .Select(static stat => new ContainerStat(
                Guid.Empty,
                stat.MemoryActive,
                stat.MemoryCache,
                stat.CpuUsage,
                stat.MemoryLimit,
                stat.RxBytes,
                stat.TxBytes,
                stat.Created))
            .ToArray();
        return Result.Success(new SwarmServiceStatsResult(
            service.DockerServiceId,
            observedTasks,
            expectedTasks,
            observedTasks == expectedTasks && missingNodeIds.Count == 0,
            candidateContainers.Select(static container => container.Id).ToArray(),
            missingNodeIds.Order(StringComparer.Ordinal).ToArray(),
            sampleTimes.Count == 0 ? null : DateTimeOffset.FromUnixTimeSeconds(sampleTimes.Min()),
            sampleTimes.Count == 0 ? null : DateTimeOffset.FromUnixTimeSeconds(sampleTimes.Max()),
            aggregate));
    }

    internal static IReadOnlyList<SwarmTaskProjection> SelectCurrentRunningTasks(
        IReadOnlyList<SwarmTaskProjection> tasks)
        => tasks
            .Where(static task => !task.IsStale
                                  && task.State.Equals("running", StringComparison.OrdinalIgnoreCase)
                                  && task.DesiredState.Equals("running", StringComparison.OrdinalIgnoreCase))
            .GroupBy(
                static task => task.Slot is { } slot
                    ? $"slot:{slot}"
                    : $"node:{task.DockerNodeId}",
                StringComparer.Ordinal)
            .Select(static group => group
                .OrderByDescending(static task => task.StatusTimestamp ?? task.ObservedAt)
                .First())
            .ToArray();

    private sealed class RuntimeIdentityComparer : IEqualityComparer<(string DockerNodeId, string DockerContainerId)>
    {
        public static RuntimeIdentityComparer Instance { get; } = new();

        public bool Equals(
            (string DockerNodeId, string DockerContainerId) left,
            (string DockerNodeId, string DockerContainerId) right) =>
            string.Equals(left.DockerNodeId, right.DockerNodeId, StringComparison.Ordinal)
            && string.Equals(left.DockerContainerId, right.DockerContainerId, StringComparison.OrdinalIgnoreCase);

        public int GetHashCode((string DockerNodeId, string DockerContainerId) value) =>
            HashCode.Combine(
                StringComparer.Ordinal.GetHashCode(value.DockerNodeId),
                StringComparer.OrdinalIgnoreCase.GetHashCode(value.DockerContainerId));
    }
}
