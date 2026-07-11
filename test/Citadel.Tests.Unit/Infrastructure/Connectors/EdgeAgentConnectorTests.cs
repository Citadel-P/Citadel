using System.Runtime.CompilerServices;
using System.Text;
using Citadel.Containers.V1;
using Citadel.Platforms.V1;
using Citadel.SharedModels.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Google.Protobuf;
using Infrastructure.Connectors.EdgeAgentConnectors;
using LightResults;
using DomainPruneResource = Domain.PruneResource;
using ProtoPruneResource = Citadel.Platforms.V1.PruneResource;

namespace Tests.Unit.Infrastructure.Connectors;

public class EdgeAgentConnectorTests
{
    [Fact]
    public async Task ListImagesAsync_Should_Route_ImageList_Command()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter
        {
            UnaryResult = EdgeAgentCommandRouterResult.Success(new ListImageResponse
            {
                Images =
                {
                    new ImageReply
                    {
                        Id = "sha256:test",
                        Created = 123,
                        RepoTags = { "alpine:latest" },
                        Size = 42
                    }
                }
            }.ToByteArray())
        };
        var connector = new EdgeImageConnector(router);

        var result = await connector.ListImagesAsync($"edge://{platformId}", CancellationToken.None);

        Assert.True(result.IsSuccess(out var images, out var error), error?.Message);
        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.ImageList, router.Kind);
        Assert.Single(images);
        Assert.Equal("sha256:test", images[0].Id);
    }

    [Fact]
    public async Task StreamStatsAsync_Should_Route_PlatformStats_Stream()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter();
        router.StreamItems.Add(EdgeAgentStreamItem.Output(new PlatformStatsResponse
        {
            CpuCount = 8,
            MemTotal = 1024,
            AgentVersion = "edge-test",
            Stat = new PlatformStatMessage
            {
                CpuUsage = 0.5,
                MemoryUsage = 100,
                ContainerCount = 2,
                ContainersRunning = 1
            }
        }.ToByteArray()));
        router.StreamItems.Add(EdgeAgentStreamItem.Complete());
        var connector = new EdgePlatformConnector(router);

        var results = new List<PlatformStatsResult>();
        await foreach (var stat in connector.StreamStatsAsync(
                           new StreamPlatformStatsCommand($"edge://{platformId}", FetchIntervalMs: 250),
                           CancellationToken.None))
        {
            results.Add(stat);
        }

        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.PlatformStatsStream, router.Kind);
        Assert.NotNull(router.Payload);
        Assert.Equal(250, PlatformStatsRequest.Parser.ParseFrom(router.Payload).FetchIntervalMs);
        var result = Assert.Single(results);
        Assert.Equal(0.5, result.PlatformStat.CpuUsage);
        Assert.Equal("edge-test", result.AgentVersion);
    }

    [Fact]
    public async Task PruneAsync_Should_Route_PlatformPrune_Command()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter
        {
            UnaryResult = EdgeAgentCommandRouterResult.Success(new PruneResponse
            {
                Resource = ProtoPruneResource.Image,
                SpaceReclaimed = 1234,
                ImagesDeleted = { "sha256:layer" }
            }.ToByteArray())
        };
        var connector = new EdgePlatformConnector(router);

        var result = await connector.PruneAsync(
            new PrunePlatformCommand($"edge://{platformId}", DomainPruneResource.Image),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out var prune, out var error), error?.Message);
        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.PlatformPrune, router.Kind);
        Assert.NotNull(router.Payload);
        Assert.Equal(ProtoPruneResource.Image, PruneRequest.Parser.ParseFrom(router.Payload).Resource);
        Assert.Equal(DomainPruneResource.Image, prune.Resource);
        Assert.Equal(1234, prune.SpaceReclaimed);
        Assert.Equal("sha256:layer", Assert.Single(prune.ImagesDeleted));
    }

    [Fact]
    public async Task ExecAsync_Should_Start_Interactive_ContainerExec_Command()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter();
        var connector = new EdgeContainerConnector(router);

        var session = await connector.ExecAsync($"edge://{platformId}", "container-1", "sh", CancellationToken.None);
        await session.SendAsync(Encoding.UTF8.GetBytes("echo test"), CancellationToken.None);
        await session.ResizeAsync(120, 40, CancellationToken.None);
        await session.DisposeAsync();

        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.ContainerExec, router.Kind);
        Assert.NotNull(router.Payload);
        var open = ExecClientMessage.Parser.ParseFrom(router.Payload).Open;
        Assert.Equal("container-1", open.ContainerId);
        Assert.Equal("sh", Assert.Single(open.Cmd));

        Assert.Equal(2, router.Inputs.Count);
        Assert.Equal("echo test", Encoding.UTF8.GetString(ExecClientMessage.Parser.ParseFrom(router.Inputs[0]).Stdin.Data.Span));
        Assert.Equal(120, ExecClientMessage.Parser.ParseFrom(router.Inputs[1]).Resize.Cols);
        Assert.True(router.Cancelled);
    }

    private sealed class TestEdgeAgentCommandRouter : IEdgeAgentCommandRouter
    {
        public Guid? PlatformId { get; private set; }
        public EdgeAgentCommandKind? Kind { get; private set; }
        public byte[]? Payload { get; private set; }
        public EdgeAgentCommandRouterResult UnaryResult { get; init; } = EdgeAgentCommandRouterResult.Success([]);
        public List<EdgeAgentStreamItem> StreamItems { get; } = [];
        public List<byte[]> Inputs { get; } = [];
        public bool Cancelled { get; private set; }

        public Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
            Guid platformId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Kind = kind;
            Payload = payload;
            return Task.FromResult(UnaryResult);
        }

        public async IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
            Guid platformId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Kind = kind;
            Payload = payload;

            foreach (var item in StreamItems)
            {
                cancellationToken.ThrowIfCancellationRequested();
                yield return item;
            }

            await Task.CompletedTask;
        }

        public Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
            Guid platformId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Kind = kind;
            Payload = payload;

            return Task.FromResult(Result.Success(new EdgeAgentInteractiveCommand("command-1", ReadStreamItemsAsync())));
        }

        public Task<Result> SendStreamInputAsync(
            Guid platformId,
            string commandId,
            byte[] payload,
            CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Inputs.Add(payload);
            return Task.FromResult(Result.Success());
        }

        public Task<Result> CancelAsync(Guid platformId, string commandId, string reason, CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Cancelled = true;
            return Task.FromResult(Result.Success());
        }

        private async IAsyncEnumerable<EdgeAgentStreamItem> ReadStreamItemsAsync()
        {
            foreach (var item in StreamItems)
            {
                yield return item;
            }

            await Task.CompletedTask;
        }
    }
}
