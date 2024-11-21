using Contracts.Broker.Models;
using EasyNetQ.AutoSubscribe;
using Microsoft.Extensions.Logging;
using Infrastructure.Services.Abstractions;

namespace Infrastructure.Services.AutoSubscribers;

/// <summary>
/// A RabbitMq subscriber that listen to Docker Daemon events
/// </summary>
internal sealed class DaemonEventAutoSubscriber(IContainerService containerService, ILogger<DaemonEventAutoSubscriber> logger) : IConsumeAsync<EventMessage>
{
    [ForTopic(EventMessage.ForTopic)]
    public async Task ConsumeAsync(EventMessage message, CancellationToken cancellationToken = default)
    {
        try
        {
            logger.LogInformation("Received message from broker: {MessageName}", nameof(EventMessage));
            switch (message.Type)
            {
                case "container": HandleContainerEvent(message, cancellationToken); break;
                case "network": HandleNetworkEvent(message, cancellationToken); break;
                case "volume": HandleVolumeEvent(message, cancellationToken); break;
                case "image": HandleImageEvent(message, cancellationToken); break;
                case "service": HandleServiceEvent(message, cancellationToken); break;
                case "node": HandleNodeEvent(message, cancellationToken); break;
                case "secret": HandleSecretEvent(message, cancellationToken); break;
                case "config": HandleConfigEvent(message, cancellationToken); break;
            }
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred: {Message}", ex.Message);
        }
    }

    private static void HandleContainerEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        switch (message.Action)
        {
            case "create": break;
            case "destroy": break;
        }
    }

    private void HandleNetworkEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleVolumeEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleImageEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleServiceEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleNodeEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleSecretEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }

    private void HandleConfigEvent(EventMessage message, CancellationToken cancellationToken = default)
    {
        throw new NotImplementedException();
    }
}