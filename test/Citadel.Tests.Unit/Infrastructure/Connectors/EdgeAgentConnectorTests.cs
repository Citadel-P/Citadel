using System.Runtime.CompilerServices;
using System.Text;
using Citadel.Containers.V1;
using Citadel.Images.V1;
using Citadel.Platforms.V1;
using Citadel.SharedModels.V1;
using Citadel.Swarm.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Swarm;
using Google.Protobuf;
using Infrastructure.Connectors.EdgeAgentConnectors;
using LightResults;
using DomainPruneResource = Domain.PruneResource;
using ProtoPruneResource = Citadel.Platforms.V1.PruneResource;

namespace Tests.Unit.Infrastructure.Connectors;

public class EdgeAgentConnectorTests
{
    [Fact]
    public async Task ListNodesAsync_ShouldRouteSwarmNodeListCommand()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter
        {
            UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmNodesResponse
            {
                Nodes =
                {
                    new SwarmNodeMessage
                    {
                        Id = "node-1",
                        Hostname = "manager-1",
                        Role = "Manager",
                        Status = "Ready",
                        Availability = "Active"
                    }
                }
            }.ToByteArray())
        };
        var connector = new EdgeSwarmConnector(router);

        var result = await connector.ListNodesAsync(
            new ListSwarmNodesCommand($"edge://{platformId}"),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out var nodes, out var error), error?.Message);
        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.SwarmNodeList, router.Kind);
        Assert.Single(nodes);
        Assert.Equal("node-1", nodes[0].Id);
    }

    [Fact]
    public async Task SwarmInventoryLists_ShouldRouteTheirMatchingEdgeCommands()
    {
        var platformId = Guid.CreateVersion7();
        var address = $"edge://{platformId}";
        var router = new TestEdgeAgentCommandRouter();
        var connector = new EdgeSwarmConnector(router);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmServicesResponse().ToByteArray());
        Assert.True((await connector.ListServicesAsync(new ListSwarmServicesCommand(address))).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmServiceList, router.Kind);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmTasksResponse().ToByteArray());
        Assert.True((await connector.ListTasksAsync(new ListSwarmTasksCommand(address))).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmTaskList, router.Kind);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmNetworksResponse().ToByteArray());
        Assert.True((await connector.ListNetworksAsync(new ListSwarmNetworksCommand(address))).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmNetworkList, router.Kind);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmSecretsResponse().ToByteArray());
        Assert.True((await connector.ListSecretsAsync(new ListSwarmSecretsCommand(address))).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmSecretList, router.Kind);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new ListSwarmConfigsResponse().ToByteArray());
        Assert.True((await connector.ListConfigsAsync(new ListSwarmConfigsCommand(address))).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmConfigList, router.Kind);
        Assert.Equal(platformId, router.PlatformId);
    }

    [Fact]
    public async Task SwarmSecretAndConfigMutations_ShouldRouteMatchingEdgeCommandsAndPayloads()
    {
        var platformId = Guid.CreateVersion7();
        var address = $"edge://{platformId}";
        var router = new TestEdgeAgentCommandRouter();
        var connector = new EdgeSwarmConnector(router);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.CreateSecretAsync(new CreateSwarmSecretCommand(
            address, "database-password", "sensitive"u8.ToArray(), new Dictionary<string, string> { ["team"] = "ops" }),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmSecretCreate, router.Kind);
        var createSecret = CreateSwarmSecretRequest.Parser.ParseFrom(router.Payload);
        Assert.Equal("database-password", createSecret.Name);
        Assert.Equal("sensitive", createSecret.Data.ToStringUtf8());
        Assert.Equal("ops", createSecret.Labels["team"]);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.UpdateSecretLabelsAsync(new UpdateSwarmSecretLabelsCommand(
            address, "secret-created", 1, new Dictionary<string, string> { ["team"] = "platform" }),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmSecretUpdate, router.Kind);
        var updateSecret = UpdateSwarmResourceLabelsRequest.Parser.ParseFrom(router.Payload);
        Assert.Equal("secret-created", updateSecret.ResourceId);
        Assert.Equal(1UL, updateSecret.VersionIndex);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.DeleteSecretAsync(
            new DeleteSwarmSecretCommand(address, "secret-created"),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmSecretDelete, router.Kind);
        Assert.Equal("secret-created", DeleteSwarmSecretRequest.Parser.ParseFrom(router.Payload).SecretId);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.CreateConfigAsync(new CreateSwarmConfigCommand(
            address, "application-config", "setting=true"u8.ToArray(), new Dictionary<string, string>()),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmConfigCreate, router.Kind);
        Assert.Equal("setting=true", CreateSwarmConfigRequest.Parser.ParseFrom(router.Payload).Data.ToStringUtf8());

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.UpdateConfigLabelsAsync(new UpdateSwarmConfigLabelsCommand(
            address, "config-created", 1, new Dictionary<string, string> { ["team"] = "platform" }),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmConfigUpdate, router.Kind);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success([]);
        Assert.True((await connector.DeleteConfigAsync(
            new DeleteSwarmConfigCommand(address, "config-created"),
            TestContext.Current.CancellationToken)).IsSuccess());
        Assert.Equal(EdgeAgentCommandKind.SwarmConfigDelete, router.Kind);
        Assert.Equal("config-created", DeleteSwarmConfigRequest.Parser.ParseFrom(router.Payload).ConfigId);
        Assert.Equal(platformId, router.PlatformId);
    }

    [Fact]
    public async Task GetConfigDataAsync_ShouldRouteAndDecodeTheEdgeResponse()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter
        {
            UnaryResult = EdgeAgentCommandRouterResult.Success(new SwarmConfigDataResponse
            {
                Data = ByteString.CopyFromUtf8("setting=true\n")
            }.ToByteArray())
        };
        var connector = new EdgeSwarmConnector(router);

        var result = await connector.GetConfigDataAsync(
            new InspectSwarmConfigCommand($"edge://{platformId}", "config-1"),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var data, out var error), error?.Message);
        Assert.Equal("setting=true\n", Encoding.UTF8.GetString(data));
        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.SwarmConfigData, router.Kind);
        Assert.Equal("config-1", InspectSwarmConfigRequest.Parser.ParseFrom(router.Payload).ConfigId);
    }

    [Fact]
    public async Task SwarmLogs_ShouldRouteBoundedServiceAndTaskCommands()
    {
        var platformId = Guid.CreateVersion7();
        var address = $"edge://{platformId}";
        var router = new TestEdgeAgentCommandRouter();
        var connector = new EdgeSwarmConnector(router);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new SwarmLogsResponse
        {
            Lines = { "service output" }
        }.ToByteArray());
        var serviceResult = await connector.GetServiceLogsAsync(
            new GetSwarmServiceLogsCommand(address, "service-1", 25),
            TestContext.Current.CancellationToken);

        Assert.True(serviceResult.IsSuccess(out var serviceLogs, out var serviceError), serviceError?.Message);
        Assert.Equal("service output", Assert.Single(serviceLogs.Lines));
        Assert.Equal(EdgeAgentCommandKind.SwarmServiceLogs, router.Kind);
        var serviceRequest = SwarmLogsRequest.Parser.ParseFrom(router.Payload);
        Assert.Equal("service-1", serviceRequest.ResourceId);
        Assert.Equal(25, serviceRequest.Tail);

        router.UnaryResult = EdgeAgentCommandRouterResult.Success(new SwarmLogsResponse
        {
            Lines = { "task output" },
            Truncated = true
        }.ToByteArray());
        var taskResult = await connector.GetTaskLogsAsync(
            new GetSwarmTaskLogsCommand(address, "task-1", 12),
            TestContext.Current.CancellationToken);

        Assert.True(taskResult.IsSuccess(out var taskLogs, out var taskError), taskError?.Message);
        Assert.True(taskLogs.Truncated);
        Assert.Equal("task output", Assert.Single(taskLogs.Lines));
        Assert.Equal(EdgeAgentCommandKind.SwarmTaskLogs, router.Kind);
        var taskRequest = SwarmLogsRequest.Parser.ParseFrom(router.Payload);
        Assert.Equal("task-1", taskRequest.ResourceId);
        Assert.Equal(12, taskRequest.Tail);
    }

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
    public async Task CheckBuildHostAsync_Should_Route_ImageCheckBuildHost_Command()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter
        {
            UnaryResult = EdgeAgentCommandRouterResult.Success(new CheckBuildHostResponse
            {
                Available = true,
                DockerVersion = "28.0.0",
                ApiVersion = "1.49",
                OperatingSystem = "linux",
                Architecture = "amd64",
                BuildKitVersion = "v0.20.2"
            }.ToByteArray())
        };
        var connector = new EdgeImageConnector(router);

        var result = await connector.CheckBuildHostAsync($"edge://{platformId}", CancellationToken.None);

        Assert.True(result.IsSuccess(out var capabilities, out var error), error?.Message);
        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.ImageCheckBuildHost, router.Kind);
        Assert.NotNull(router.Payload);
        Google.Protobuf.WellKnownTypes.Empty.Parser.ParseFrom(router.Payload);
        Assert.True(capabilities.Available);
        Assert.Equal("28.0.0", capabilities.DockerVersion);
        Assert.Equal("1.49", capabilities.ApiVersion);
        Assert.Equal("linux", capabilities.OperatingSystem);
        Assert.Equal("amd64", capabilities.Architecture);
        Assert.Equal("v0.20.2", capabilities.BuildKitVersion);
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
                ContainersRunning = 1,
                DiskUsedBytes = 75,
                DiskTotalBytes = 100,
                DiskUsage = 75
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
        Assert.Equal(75, result.PlatformStat.DiskUsedBytes);
        Assert.Equal(100, result.PlatformStat.DiskTotalBytes);
        Assert.Equal(75, result.PlatformStat.DiskUsage);
        Assert.Equal("edge-test", result.AgentVersion);
    }

    [Fact]
    public async Task StreamDaemonEventAsync_ShouldMapSwarmResourceEventAndScope()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter();
        router.StreamItems.Add(EdgeAgentStreamItem.Output(new DaemonEventResponse
        {
            Scope = Citadel.Platforms.V1.DaemonEventScope.SwarmScope,
            DaemonResourceEventResponse = new DaemonResourceEventResponse
            {
                Type = EventMessageType.Secret,
                Action = "create",
                ResourceId = "secret-1"
            }
        }.ToByteArray()));
        router.StreamItems.Add(EdgeAgentStreamItem.Complete());
        var connector = new EdgePlatformConnector(router);

        var events = new List<DaemonEventInfo>();
        await foreach (var daemonEvent in connector.StreamDaemonEventAsync(
                           new StreamDaemonEventCommand($"edge://{platformId}"),
                           TestContext.Current.CancellationToken))
        {
            events.Add(daemonEvent);
        }

        var resource = Assert.IsType<DaemonResourceEventInfo>(Assert.Single(events));
        Assert.Equal(ContainerEventType.Secret, resource.Type);
        Assert.Equal("create", resource.Action);
        Assert.Equal("secret-1", resource.ResourceId);
        Assert.Equal(global::Domain.Contracts.Resources.Containers.DaemonEventScope.Swarm, resource.Scope);
        Assert.Equal(EdgeAgentCommandKind.PlatformDaemonEventsStream, router.Kind);
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

    [Fact]
    public async Task ExecBinaryAsync_Should_Route_ContainerExecBinary_Stream()
    {
        var platformId = Guid.CreateVersion7();
        var router = new TestEdgeAgentCommandRouter();
        router.StreamItems.Add(EdgeAgentStreamItem.Output(new ExecServerMessage
        {
            Output = new ExecOutput
            {
                Data = ByteString.CopyFromUtf8("out"),
                Stream = StreamType.Stdout
            }
        }.ToByteArray()));
        router.StreamItems.Add(EdgeAgentStreamItem.Output(new ExecServerMessage
        {
            Output = new ExecOutput
            {
                Data = ByteString.CopyFromUtf8("err"),
                Stream = StreamType.Stderr
            }
        }.ToByteArray()));
        router.StreamItems.Add(EdgeAgentStreamItem.Output(new ExecServerMessage
        {
            Exit = new ExecExit { ExitCode = 7 }
        }.ToByteArray()));
        router.StreamItems.Add(EdgeAgentStreamItem.Complete());
        var connector = new EdgeContainerConnector(router);

        var result = await connector.ExecBinaryAsync(
            $"edge://{platformId}",
            new ContainerBinaryExecRequest("container-1", ["volume-helper", "stream-file"], Environment: new Dictionary<string, string> { ["A"] = "B" }),
            CancellationToken.None);

        Assert.True(result.IsSuccess(out var exec, out var error), error?.Message);
        var chunks = new List<ContainerBinaryExecChunk>();
        await foreach (var chunk in exec.Output)
        {
            chunks.Add(chunk);
        }

        var exitCode = await exec.GetExitCodeAsync(CancellationToken.None);

        Assert.Equal(platformId, router.PlatformId);
        Assert.Equal(EdgeAgentCommandKind.ContainerExecBinary, router.Kind);
        Assert.NotNull(router.Payload);
        var request = ExecBinaryRequest.Parser.ParseFrom(router.Payload);
        Assert.Equal("container-1", request.ContainerId);
        Assert.Equal(["volume-helper", "stream-file"], request.Cmd);
        Assert.Equal("B", request.Env["A"]);
        Assert.Equal(2, chunks.Count);
        Assert.Equal(ContainerExecStream.Stdout, chunks[0].Stream);
        Assert.Equal("out", Encoding.UTF8.GetString(chunks[0].Data.Span));
        Assert.Equal(ContainerExecStream.Stderr, chunks[1].Stream);
        Assert.Equal("err", Encoding.UTF8.GetString(chunks[1].Data.Span));
        Assert.Equal(7, exitCode);
    }

    private sealed class TestEdgeAgentCommandRouter : IEdgeAgentCommandRouter
    {
        public Guid? PlatformId { get; private set; }
        public EdgeAgentCommandKind? Kind { get; private set; }
        public byte[]? Payload { get; private set; }
        public EdgeAgentCommandRouterResult UnaryResult { get; set; } = EdgeAgentCommandRouterResult.Success([]);
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

        public Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
            EdgeAgentResourceType resourceType,
            Guid resourceId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
            => SendUnaryAsync(resourceId, kind, payload, timeout, correlationId, cancellationToken);

        public Task<EdgeAgentCommandRouterResult> SendUnaryAsync(
            Guid platformId,
            string dockerNodeId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
            => SendUnaryAsync(platformId, kind, payload, timeout, correlationId, cancellationToken);

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

        public async IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
            EdgeAgentResourceType resourceType,
            Guid resourceId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await foreach (var item in SendServerStreamAsync(resourceId, kind, payload, timeout, correlationId, cancellationToken))
            {
                yield return item;
            }
        }

        public async IAsyncEnumerable<EdgeAgentStreamItem> SendServerStreamAsync(
            Guid platformId,
            string dockerNodeId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            [EnumeratorCancellation] CancellationToken cancellationToken)
        {
            await foreach (var item in SendServerStreamAsync(
                               platformId,
                               kind,
                               payload,
                               timeout,
                               correlationId,
                               cancellationToken))
            {
                yield return item;
            }
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

        public Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
            EdgeAgentResourceType resourceType,
            Guid resourceId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
            => StartInteractiveAsync(resourceId, kind, payload, timeout, correlationId, cancellationToken);

        public Task<Result<EdgeAgentInteractiveCommand>> StartInteractiveAsync(
            Guid platformId,
            string dockerNodeId,
            EdgeAgentCommandKind kind,
            byte[] payload,
            TimeSpan timeout,
            string? correlationId,
            CancellationToken cancellationToken)
            => StartInteractiveAsync(platformId, kind, payload, timeout, correlationId, cancellationToken);

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

        public Task<Result> SendStreamInputAsync(
            Guid platformId,
            string dockerNodeId,
            string commandId,
            byte[] payload,
            CancellationToken cancellationToken)
            => SendStreamInputAsync(platformId, commandId, payload, cancellationToken);

        public Task<Result> CancelAsync(Guid platformId, string commandId, string reason, CancellationToken cancellationToken)
        {
            PlatformId = platformId;
            Cancelled = true;
            return Task.FromResult(Result.Success());
        }

        public Task<Result> CancelAsync(
            Guid platformId,
            string dockerNodeId,
            string commandId,
            string reason,
            CancellationToken cancellationToken)
            => CancelAsync(platformId, commandId, reason, cancellationToken);

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
