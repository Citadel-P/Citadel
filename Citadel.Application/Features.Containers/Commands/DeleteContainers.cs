using System.Collections.Concurrent;
using Agent.Server.Containers;
using FluentValidation;
using Google.Api;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Containers.Commands;

public sealed record DeleteContainers(string[] ContainersIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

internal sealed class DeleteContainersHandler(
    IGrpcClientFactory clientFactory,
    ApplicationDbContext dbContext,
    ILogger<DeleteContainersHandler> logger) : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken cancellationToken)
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

        if (platforms.Count == 0)
        {
            return Result.Failure(new NotFoundError("No containers found for the given IDs."));
        }

        var exceptions = new Exception[platforms.Sum(s => s.ContainersId.Count())];
        var exceptionIndex = 0;
        var parallelOptions = new ParallelOptions
        {
            MaxDegreeOfParallelism = Environment.ProcessorCount,
            CancellationToken = cancellationToken
        };
        await Parallel.ForEachAsync(platforms, parallelOptions, async (platform, ct) =>
        {
            var client = clientFactory.GetContainerClient(platform.Address);
            try
            {
                var message = new DeleteContainersMessage
                {
                    Ids = { platform.ContainersId },
                    V = request.V ?? false,
                    Force = request.Force ?? false,
                    Link = request.Link ?? false,
                };
                await client.DeleteContainersAsync(message, cancellationToken: ct);
            }
            catch (Exception ex)
            {
                var idx = Interlocked.Increment(ref exceptionIndex) - 1;
                if (idx < exceptions.Length)
                    exceptions[idx] = ex;
                logger.LogError(ex, "Error while processing container {ContainerIds} on platform {PlatformAddress}", platform.ContainersId, platform.Address);
            }
        });

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
                : Result.Failure(new InternalServerError($"An error occurred while processing the request, {exceptions.First().Message}"));
        }
    }
}