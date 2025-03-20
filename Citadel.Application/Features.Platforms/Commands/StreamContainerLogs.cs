using System.Runtime.CompilerServices;
using Agent.Server.Containers;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Infrastructure;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Mediator;

namespace Application.Features.Platforms.Commands;

public sealed record StreamContainerLogs(string ContainerId) : IStreamCommand<ContainerLogReply>
{
    internal class Validator : AbstractValidator<StreamContainerLogs>
    {
        public Validator()
        {
            RuleFor(s => s.ContainerId).ValidContainerId();
        }
    }
}

internal class StreamContainerLogsHandler(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext): IStreamCommandHandler<StreamContainerLogs, ContainerLogReply>
{
    public async IAsyncEnumerable<ContainerLogReply> Handle(StreamContainerLogs query, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.ContainersInfo.GetPlatformAddress(query.ContainerId, cancellationToken)
             ?? throw new Exception($"Platform doesn't exist for container {query.ContainerId}");
        
        var client = clientFactory.GetContainerClient(platformAddress);
        var request = new ContainerLogRequest
        {
            ContainerId = query.ContainerId,
        };
        using var call = client.StreamContainerLogs(request, cancellationToken: cancellationToken);
        await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return reply;
        }
    }
}