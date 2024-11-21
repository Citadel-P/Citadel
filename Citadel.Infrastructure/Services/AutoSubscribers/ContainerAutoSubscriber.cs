using Contracts.Broker.Models;
using EasyNetQ.AutoSubscribe;
using Microsoft.Extensions.Logging;
using Infrastructure.Services.Abstractions;

namespace Infrastructure.Services.AutoSubscribers;

/// <summary>
/// A RabbitMq subscriber for messages related to container's notifications
/// </summary>
public class ContainerAutoSubscriber(IContainerService containerService, ILogger<ContainerAutoSubscriber> logger) :
    IConsumeAsync<ContainersInfoMessage>,
    IConsumeAsync<ContainerLogMessage>
{
    [ForTopic(ContainersInfoMessage.ForTopic)]
    public async Task ConsumeAsync(ContainersInfoMessage message, CancellationToken cancellationToken = default)
    {
        try
        {
            logger.LogInformation("Received message from broker: {MessageName}", nameof(ContainersInfoMessage));
            await containerService.OnContainersInfoMessage(message, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred: {Message}", ex.Message);
        }
    }

    [ForTopic(ContainerLogMessage.ForTopic)]
    public async Task ConsumeAsync(ContainerLogMessage message, CancellationToken cancellationToken = default)
    {
        try
        {
            logger.LogInformation("Received message from broker: {MessageName}", nameof(ContainersInfoMessage));
            await containerService.OnContainerLogsMessage(message, cancellationToken);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred: {Message}", ex.Message);
        }
    }
}