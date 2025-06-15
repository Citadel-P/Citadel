using System.Threading.Channels;
using EFCore.BulkExtensions;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Domain;
using Domain.Contracts.Resources.Platforms;

namespace Infrastructure.TaskJobs;

internal class PlatformsStatsPersistenceJob(
    IServiceScopeFactory scopeFactory,
    ChannelReader<PlatformStatsBatch> reader,
    ISignalRConnectionTracker connectionTracker,
    ILogger<PlatformsStatsPersistenceJob> logger) : BackgroundService
{
    private const int BatchSize = 200;
    // Updates to db will be flushed every x seconds or when batch size is reached.
    private static readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(60*2); 

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var buffer = new Dictionary<Guid, List<PlatformStat>>(); 
        var lastFlush = DateTime.UtcNow;

        try
        {
            await foreach (var batch in reader.ReadAllAsync(cancellationToken))
            {
                // Add to buffer
                if (!buffer.TryGetValue(batch.PlatformId, out var list))
                {
                    list = [];
                    buffer[batch.PlatformId] = list;
                }
                list.Add(batch.PlatformStat);

                // Flush if batch size exceeded or interval exceeded
                int totalCount = buffer.Sum(x => x.Value.Count);
                if (totalCount >= BatchSize || DateTime.UtcNow - lastFlush >= FlushInterval)
                {
                    await SaveBatchToDb(buffer, batch, cancellationToken);
                    buffer.Clear();
                    lastFlush = DateTime.UtcNow;
                }

                // Push to clients
                await NotifyClients(batch);
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
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<PlatformStat>> statsByPlatform, PlatformStatsBatch batch, CancellationToken cancellationToken)
    {
        using var scope = scopeFactory.CreateAsyncScope();
        using var db = scope.ServiceProvider.GetRequiredService<ApplicationDbContext>();
        foreach (var (platformId, stats) in statsByPlatform)
        {
            var existing = await db.Platforms.FirstOrDefaultAsync(s => s.Id == platformId, cancellationToken);
            if (existing == null)
            {
                logger.LogWarning("Platform with ID {PlatformId} not found in the database.", platformId);
                continue;
            }

            PlatformDescriptor? descriptor = null;
            if (existing.PlatformDescriptor is DockerPlatformDescriptor dockerPlatform)
            {
                descriptor = dockerPlatform.Create(
                    containersRunning: batch.ContainersRunning,
                    containersPaused: batch.ContainersPaused,
                    containersStopped: batch.ContainersStopped);
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
                networkCount: batch.NetworkCount,
                volumeCount: batch.VolumeCount,
                imageCount: batch.ImageCount,
                memTotal: batch.MemTotal,
                descriptor: descriptor);
        }

        try
        {
            var stats = statsByPlatform.Values.SelectMany(s => s).ToArray();
            await db.BulkInsertAsync(stats, cancellationToken: cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to save platform stats batch to the database. Batch size: {BatchSize}", statsByPlatform.Sum(s => s.Value.Count));
        }
    }

    private async ValueTask NotifyClients(PlatformStatsBatch batch)
    {
        if (connectionTracker.HasUsersInGroup("Platforms"))
        {
            using var scope = scopeFactory.CreateAsyncScope();
            var hub = scope.ServiceProvider.GetRequiredService<IPlatformHubDispatcher>();
            try
            {
                await hub.PushPlatformStats(batch);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to push platform stats to clients for platform {PlatformId}", batch.PlatformId);
                return;
            }
        }
    }
}
