using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Hosting.Common;
using Infrastructure.EntityFramework;
using static Agent.Server.Containers.Containers;
using Agent.Server.Containers;
using Infrastructure.Services.Abstractions;
using Google.Protobuf.WellKnownTypes;
using Hosting.Common.ErrorTypes;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

public sealed record PatchContainers(string[] ContainersIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

public enum ContainerAction : uint
{
    START = 0,
    RESTART,
    STOP,
    PAUSE,
    UNPAUSE,
    DELETE
}

internal class PatchContainersHandler(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<PatchContainersHandler> logger)
    : ICommandHandler<PatchContainers, Result>
{
    public async ValueTask<Result> Handle(PatchContainers request, CancellationToken cancellationToken)
    {
        var containers = await dbContext.ContainersInfo.Include(s => s.Platform)
                        .AsNoTracking()
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .ToListAsync(cancellationToken);
        
        Parallel.ForEach(containers.GroupBy(s => s.Platform.Address), async container =>
        {
            var containerIds = container.Select(s => s.ContainerId);
            var client = clientFactory.GetContainerClient(container.Key);
            try
            {
                await ToOperation(client, containerIds, request.Action);
            }
            catch (Exception ex)
            {
                logger.LogError(ex, "Error while processing container {ContainerIds} on platform {PlatformAddress}", containerIds, container.Key);
            }
        });

        async Task<Empty> ToOperation(ContainersClient client, IEnumerable<string> containersIds, ContainerAction action) => action switch
        {
            ContainerAction.START => await client.StartContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.STOP => await client.StopContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.PAUSE => await client.PauseContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.UNPAUSE => await client.UnpauseContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.DELETE => await client.DeleteContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.RESTART => await client.RestartContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            _ => throw new NotImplementedException()
        };

        return Result.Success();
    }
}