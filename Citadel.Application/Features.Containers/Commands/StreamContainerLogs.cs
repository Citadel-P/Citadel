using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Infrastructure;
using Infrastructure.EntityFramework;
using Mediator;

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

internal class StreamContainerLogsHandler(IContainerService containerService, ApplicationDbContext dbContext)
    : IStreamCommandHandler<StreamContainerLogs, ContainerLogInfo>
{
    public async IAsyncEnumerable<ContainerLogInfo> Handle(StreamContainerLogs query, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Containers.GetPlatformAddress(query.ContainerId, cancellationToken)
             ?? throw new Exception($"Platform doesn't exist for container {query.ContainerId}");

        var command = new StreamContainerLogsCommand
            (
                ContainerId: query.ContainerId, 
                PlatformAddress: platformAddress
            );
        await foreach(var log in containerService.StreamLogsAsync(command, cancellationToken))
        {
            yield return log;
        }
    }
}