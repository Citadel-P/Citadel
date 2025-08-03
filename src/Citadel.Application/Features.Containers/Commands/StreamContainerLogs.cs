using System.Runtime.CompilerServices;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Containers.Commands;

public sealed record StreamContainerLogs(string ContainerId) : IStreamCommand<ContainerLogInfo>
{
    internal class Validator : AbstractValidator<StreamContainerLogs>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerId).ValidContainerId();
        }
    }
}

internal class StreamContainerLogsHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IContainerConnector> connectorFactory, ILogger<StreamContainerLogsHandler> logger)
    : IStreamCommandHandler<StreamContainerLogs, ContainerLogInfo>
{
    public async IAsyncEnumerable<ContainerLogInfo> Handle(StreamContainerLogs query, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetPlatformByContainerId(query.ContainerId, out var platform))
        {
            logger.LogError("No platform found for container ID {ContainerId}", query.ContainerId);
            yield break; // No platform found for the given container ID
        }

        var command = new StreamContainerLogsCommand
        (
            ContainerId: query.ContainerId,
            PlatformAddress: platform.Address
        );
        await foreach(var log in connectorFactory.GetConnector(platform.ConnectorType).StreamLogsAsync(command, cancellationToken))
        {
            yield return log;
        }
    }
}