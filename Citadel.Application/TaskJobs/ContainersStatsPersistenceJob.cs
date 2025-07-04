using System.Threading.Channels;
using Application.Configs;
using Application.Services.Abstractions;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal class ContainersStatsPersistenceJob(
    IServiceScopeFactory scopeFactory,
    IOptions<JobConfiguration> options,
    ChannelReader<ContainersStatBatch> reader,
    ISignalRConnectionTracker connectionTracker,
    IContainerHubDispatcher containerHubDispatcher,
    ILogger<ContainersStatsPersistenceJob> logger) : BackgroundService
{
    private readonly int BatchSize = options.Value?.BatchSize ?? 500;
    // Updates to db will be flushed every x seconds or when batch size is reached.
    private readonly TimeSpan FlushInterval = TimeSpan.FromSeconds(options.Value?.FlashInterval ?? 60); 

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        var buffer = new Dictionary<Guid, List<ContainerStat>>(); // Key: PlatformId
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
                list.AddRange(batch.Stats);

                // Flush if batch size exceeded or interval exceeded
                int totalCount = buffer.Sum(x => x.Value.Count);
                if (totalCount >= BatchSize || DateTime.UtcNow - lastFlush >= FlushInterval)
                {
                    await SaveBatchToDb(buffer, cancellationToken);
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
            logger.LogError(ex, $"Error in {nameof(ContainersStatsPersistenceJob)}");
        }

        // Final flush
        if (buffer.Count > 0)
        {
            await SaveBatchToDb(buffer, CancellationToken.None);
        }
    }

    private async Task SaveBatchToDb(Dictionary<Guid, List<ContainerStat>> statsByPlatform, CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

            var stats = statsByPlatform.SelectMany(s => s.Value).ToList();
            await uow.ContainerStats.BulkInsertAsync(stats, cancellationToken);
            await uow.CommitAsync();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Failed to save container stats to the database.");
        }
    }

    private async ValueTask NotifyClients(ContainersStatBatch batch)
    {
        if (connectionTracker.HasUsersInGroup($"ContainersInfo/{batch.PlatformId}"))
        {
            try
            {
                await containerHubDispatcher.SendContainersStats(batch.PlatformId, batch.Stats);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Failed to notify clients about containers stats for platform {PlatformId}", batch.PlatformId);
            }
        }
    }
}
