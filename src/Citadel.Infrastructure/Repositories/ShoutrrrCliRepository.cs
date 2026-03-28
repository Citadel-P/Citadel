using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories.Mappers;
using LightResults;
using Microsoft.Extensions.Logging;

namespace Infrastructure.Repositories;

internal class ShoutrrrCliRepository(IProcessService processService, ILogger<ShoutrrrCliRepository> logger) : IShoutrrrCliRepository
{
    private readonly string shoutrrrCliPath = "shoutrrr";

    public Task SendAlertAsync(AlertEvent alertEvent, IEnumerable<AlertChannel> channels, string name, CancellationToken cancellationToken)
    {
        var title = $"[{alertEvent.Severity}] {name}";
        var message = alertEvent.Info.HumanMessage;
        return SendToChannelsAsync(channels, title, message, cancellationToken);
    }

    public Task<Result> SendTestNotificationAsync(AlertChannel channel, CancellationToken cancellationToken)
        => VerifyAndSendTestAsync(channel, cancellationToken);

    private async Task SendToChannelsAsync(
    IEnumerable<AlertChannel> channels,
    string title,
    string message,
    CancellationToken cancellationToken)
    {
        var activeChannels = channels.Where(c => c.IsActive).ToList();

        await Parallel.ForEachAsync(activeChannels, new ParallelOptions
        {
            CancellationToken = cancellationToken,
            MaxDegreeOfParallelism = Environment.ProcessorCount / 2
        }, async (channel, ct) =>
        {
            var args = new[]
            {
                "send",
                "--url", channel.Url,
                "--title", title,
                "--message", message
            };

            var result = await processService.ExecuteAsync(shoutrrrCliPath, args, null, null, ct);

            if (!result.IsSuccess)
            {
                logger.LogWarning("Shoutrrr failed for channel {ChannelId}. ExitCode={ExitCode}. Error={Error}", channel.Id, result.ExitCode, result.StandardError);
            }
            else
            {
                logger.LogInformation("Notification sent to channel {ChannelId}. Output={Output}", channel.Id, result.StandardOutput);
            }
        });
    }

    private async Task<Result> VerifyChannelsAsync(AlertChannel channel, CancellationToken cancellationToken)
    {
        var args = new[]
            {
                "verify",
                "--url", channel.Url,
            };

        var result = await processService.ExecuteAsync(shoutrrrCliPath, args, null, null, cancellationToken);

        return result.Map();
    }

    private async Task<Result> VerifyAndSendTestAsync(AlertChannel channel, CancellationToken cancellationToken)
    {
        var verifyResult = await VerifyChannelsAsync(channel, cancellationToken);
        if (!verifyResult.IsSuccess())
        {
            return verifyResult;
        }

        var args = new[]
        {
            "send",
            "--url", channel.Url,
            "--title", "Citadel Notification Test",
            "--message", $"Test notification for channel '{channel.Name}'"
        };

        var sendResult = await processService.ExecuteAsync(shoutrrrCliPath, args, null, null, cancellationToken);
        
        return sendResult.Map();
    }
}
