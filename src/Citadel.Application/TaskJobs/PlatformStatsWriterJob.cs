using System.Threading.Channels;
using Application.Configs;
using Application.Mappers;
using Application.Services.Abstractions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

/// <summary>
/// Background service that batches and persists platforms statistics received from a channel, periodically flushing them to the database and notifying connected clients 
/// with the latest stats updates.
/// </summary>
internal class PlatformStatsWriterJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    ISignalRConnectionTracker connectionTracker,
    IPlatformHubDispatcher platformHubDispatcher,
    ChannelReader<(Guid Id, PlatformStatsResult Stats)> reader,
    ILogger<PlatformStatsWriterJob> logger) : BackgroundService
{
    private readonly int BatchSize = options.Value?.BatchSize ?? 500;
    // Updates to db will be flushed every x seconds or when platform batch size is reached.
    private readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60); 

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
            logger.LogError(ex, $"Error in {nameof(PlatformStatsWriterJob)}");
        }

        // Final flush
        if (buffer.Count > 0)
        {
            await SaveBatchToDb(buffer, CancellationToken.None);
        }
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<PlatformStatsResult>> statsByPlatform, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        foreach (var (platformId, stats) in statsByPlatform)
        {
            var existing = await uow.Platforms.GetByIdAsync(platformId, cancellationToken);
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

            await uow.Platforms.UpdatePlatformAsync(existing, cancellationToken);
            await uow.CommitAsync();
        }

        try
        {
            var batch = statsByPlatform.Map();
            await uow.PlatformStats.BulkInsertAsync(batch, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while persisting the platform statistics batch to the database. Total stats in batch: {BatchSize}", statsByPlatform.Sum(s => s.Value.Count));
        }
    }

    private async ValueTask NotifyClients(Guid platformId, PlatformStatsResult stats)
    {
        if (connectionTracker.HasUsersInGroup("Platforms"))
        {
            try
            {
                await platformHubDispatcher.PushPlatformStats(platformId, stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "An error occurred while sending platform statistics to clients for platform ID: {PlatformId}", platformId);
                return;
            }
        }
    }
}
