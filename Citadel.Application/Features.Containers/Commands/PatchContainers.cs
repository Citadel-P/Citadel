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
using Grpc.Core;
using System.Collections.Concurrent;

namespace Application.Features.Containers.Commands;

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
    UNPAUSE
}

internal class PatchContainersHandler(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<PatchContainersHandler> logger)
    : ICommandHandler<PatchContainers, Result>
{
    public async ValueTask<Result> Handle(PatchContainers request, CancellationToken cancellationToken)
    {
        var platforms = await dbContext.ContainersInfo.Include(s => s.Platform)
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .GroupBy(s => s.Platform.Address)
                        .Select(s => new 
                        { 
                            Address = s.Key, 
                            ContainersId = s.Select(x => x.ContainerId) 
                        })
                        .AsNoTracking()
                        .ToListAsync(cancellationToken);

        var exceptions = new ConcurrentBag<Exception>();
        await Parallel.ForEachAsync(platforms, cancellationToken, async (platform, token) =>
        {
            var client = clientFactory.GetContainerClient(platform.Address);
            try
            {
                await ToOperation(client, platform.ContainersId, request.Action, token);
            }
            catch (Exception ex)
            {
                exceptions.Add(ex);
                logger.LogError(ex, "Error while processing container {ContainerIds} on platform {PlatformAddress}", platform.ContainersId, platform.Address);
            }
        });

        static async Task<Empty> ToOperation(ContainersClient client, IEnumerable<string> containersIds, ContainerAction action, CancellationToken cancellationToken) => action switch
        {
            ContainerAction.START => await client.StartContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.STOP => await client.StopContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.PAUSE => await client.PauseContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.UNPAUSE => await client.UnpauseContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            ContainerAction.RESTART => await client.RestartContainersAsync(new ContainersId() { Ids = { containersIds } }, cancellationToken: cancellationToken),
            _ => throw new NotImplementedException()
        };

        if (exceptions.IsEmpty)
        {
            return Result.Success();
        }
        else
        {
            if (exceptions.Any(s => s is RpcException))
            {
                return Result.Failure(new ClientRpcException($"An RPC exception occurred: {exceptions.First(s => s is RpcException).Message}"));
            }
            else
            {
                return Result.Failure(new Exception("An error occurred while processing the request"));
            }
        }
    }
}