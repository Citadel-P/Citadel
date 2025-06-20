using System.Threading.Channels;
using Domain.Entities.Platforms;
using Application.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Interfaces;
using Application.Mappers;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platform statistics received from a channel, periodically flushing them to the database and notifying connected clients 
/// with the latest stats updates.
/// </summary>
internal class PlatformsStatsPersistenceJob(
    IServiceScopeFactory scopeFactory,
    ISignalRConnectionTracker connectionTracker,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    ILogger<PlatformsStatsPersistenceJob> logger) : BackgroundService
{
    private const int BatchSize = 200;
    // Updates to db will be flushed every x seconds or when platform size is reached.
    private static readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(60*2); 

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var buffer = new Dictionary<Guid, List<PlatformStatsResult>>(); 
        var lastFlush = DateTime.UtcNow;

        try
        {
            await foreach (var (platformId, stats) in reader.ReadAllAsync(cancellationToken))
            {
                // Add to buffer
                if (!buffer.TryGetValue(platformId, out var list))
                {
                    list = [];
                    buffer[platformId] = list;
                }
                list.Add(stats);

                // Flush if platform size exceeded or interval exceeded
                int totalCount = buffer.Sum(x => x.Value.Count);
                if (totalCount >= BatchSize || DateTime.UtcNow - lastFlush >= FlushInterval)
                {
                    await SaveBatchToDb(buffer, cancellationToken);
                    buffer.Clear();
                    lastFlush = DateTime.UtcNow;
                }

                // Push to clients
                await NotifyClients(platformId, stats);
            }
        }
        catch (OperationCanceledException)
        {
            // Nope, this is expected when the service is stopping
        }
        catch (Exception ex)
        {
            logger.LogError(ex, $"Error in {nameof(PlatformsStatsPersistenceJob)}");
        }

        // Final flush
        if (buffer.Count > 0)
        {
            await SaveBatchToDb(buffer, CancellationToken.None);
        }
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<PlatformStatsResult>> statsByPlatform, CancellationToken cancellationToken)
    {
        using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        foreach (var (platformId, stats) in statsByPlatform)
        {
            var existing = await uow.Platforms.Query().FirstOrDefaultAsync(s => s.Id == platformId, cancellationToken);
            if (existing == null)
            {
                logger.LogWarning("Platform with ID {PlatformId} not found in the database.", platformId);
                continue;
            }

            var lastBatch = stats.LastOrDefault();
            if (lastBatch == null)
                continue;

            PlatformDescriptor? descriptor = null;
            if (existing.PlatformDescriptor is DockerPlatformDescriptor dockerPlatform)
            {
                descriptor = dockerPlatform.Create(
                    containersRunning: lastBatch.ContainersRunning,
                    containersPaused: lastBatch.ContainersPaused,
                    containersStopped: lastBatch.ContainersStopped);
            }
            else if (existing.PlatformDescriptor is DockerSwarmPlatformDescriptor swarmDescriptor)
            {
                // Todo
            }
            else if (existing.PlatformDescriptor is KubernetesPlatformDescriptor kubernetesDescriptor)
            {
                // Todo
            }

            existing.PartialUpdate(
                platformStatus: PlatformStatus.Online,
                networkCount: lastBatch.NetworkCount,
                volumeCount: lastBatch.VolumeCount,
                imageCount: lastBatch.ImageCount,
                memTotal: lastBatch.MemTotal,
                descriptor: descriptor);
        }

        try
        {
            var batch = statsByPlatform.Map();
            await uow.BulkInsertAsync(batch, cancellationToken: cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to save platform stats batch to the database. Batch size: {BatchSize}", statsByPlatform.Sum(s => s.Value.Count));
        }
    }

    private async ValueTask NotifyClients(Guid platformId, PlatformStatsResult stats)
    {
        if (connectionTracker.HasUsersInGroup("Platforms"))
        {
            using var scope = scopeFactory.CreateAsyncScope();
            var hub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();
            try
            {
                await hub.PushPlatformStats(platformId, stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to push platform stats to clients for platform: {Id}", platformId);
                return;
            }
        }
    }
}
