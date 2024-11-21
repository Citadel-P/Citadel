using Contracts.Broker.Models;
using EasyNetQ.AutoSubscribe;
using Microsoft.Extensions.Logging;
using Infrastructure.Services.Abstractions;

namespace Infrastructure.Services.AutoSubscribers;

/// <summary>
/// A RabbitMq subscriber for messages related to the <see cref="Platform"/> notifications
/// </summary>
internal sealed class PlatformAutoSubscriber(IPlatformService platformService, ILogger<PlatformAutoSubscriber> logger) : IConsumeAsync<SystemInfoMessage>
{
    [ForTopic(SystemInfoMessage.ForTopic)]
    public async Task ConsumeAsync(SystemInfoMessage message, CancellationToken cancellationToken = default)
    {
        try
        {
            logger.LogInformation("Received message from broker: {MessageName}", nameof(SystemInfoMessage));
            await platformService.OnSystemInfoMessage(message, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred: {Message}", ex.Message);
        }
    }
}