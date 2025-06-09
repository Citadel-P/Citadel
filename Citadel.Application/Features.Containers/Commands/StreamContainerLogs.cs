using System.Runtime.CompilerServices;
using Citadel.Agent.Containers.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Infrastructure;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record StreamContainerLogs(string ContainerId) : IStreamCommand<ContainerLogResponse>
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
    ApplicationDbContext dbContext): IStreamCommandHandler<StreamContainerLogs, ContainerLogResponse>
{
    public async IAsyncEnumerable<ContainerLogResponse> Handle(StreamContainerLogs query, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Containers.GetPlatformAddress(query.ContainerId, cancellationToken)
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