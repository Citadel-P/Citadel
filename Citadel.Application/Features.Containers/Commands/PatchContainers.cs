using System.Collections.Concurrent;
using System.ComponentModel;
using Agent.Server.Containers;
using FluentValidation;
using Google.Protobuf.WellKnownTypes;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using static Agent.Server.Containers.Containers;

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
        var platforms = await dbContext.Containers.Include(s => s.Platform)
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .GroupBy(s => s.Platform.Address)
                        .Select(s => new 
                        { 
                            Address = s.Key, 
                            ContainersId = s.Select(x => x.ContainerId) 
                        })
                        .AsNoTracking()
                        .ToListAsync(cancellationToken);

        if (platforms.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the given IDs."));
        }

        // Pre-allocate array for results
        var exceptions = new Exception[platforms.Sum(s => s.ContainersId.Count())];
        var exceptionIndex = 0;
        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = cancellationToken
        };
        await Parallel.ForEachAsync(platforms, parallelOptions, async (platform, token) =>
        {
            var client = clientFactory.GetContainerClient(platform.Address);
            try
            {
                await ToOperation(client, platform.ContainersId, request.Action, token);
            }
            catch (Exception ex)
            {
                var idx = Interlocked.Increment(ref exceptionIndex) - 1;
                if (idx < exceptions.Length)
                    exceptions[idx] = ex;
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

        exceptions = [.. exceptions.Where(e => e is not null)]; // Filter out null exceptions

        if (exceptions.Length == 0)
        {
            return Result.Success();
        }
        else
        {
            var rpcException = exceptions.OfType<RpcException>().FirstOrDefault();
            return rpcException is not null
                ? Result.Failure(new ClientRpcException($"An RPC exception occurred: {rpcException.Message}", rpcException.StatusCode))
                : Result.Failure(new InternalServerError($"An error occurred while processing the request,  {exceptions.First().Message}"));
        }
    }
}