using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Infrastructure.EntityFramework;
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

internal class StreamContainerLogsHandler(IConnectorFactory<IContainerConnector> connectorFactory, ApplicationDbContext dbContext, 
    ILogger<StreamContainerLogsHandler> logger): IStreamCommandHandler<StreamContainerLogs, ContainerLogInfo>
{
    public async IAsyncEnumerable<ContainerLogInfo> Handle(StreamContainerLogs query, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var (address, id, connectorType) = await dbContext.Containers.GetPlatformIdAsync(query.ContainerId, cancellationToken);
        if (string.IsNullOrEmpty(address) || id is null || connectorType is null)
        {
            logger.LogError("No platform found for container ID {ContainerId}", query.ContainerId);
            yield break; // No platform found for the given container ID
        }

        var command = new StreamContainerLogsCommand
        (
            ContainerId: query.ContainerId,
            PlatformAddress: address
        );
        await foreach(var log in connectorFactory.GetConnector(connectorType.Value).StreamLogsAsync(command, cancellationToken))
        {
            yield return log;
        }
    }
}