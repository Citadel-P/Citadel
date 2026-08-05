using Citadel.Swarm.V1;
using Google.Protobuf.WellKnownTypes;
using Hosting.DockerClient.Models.Swarm;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class SwarmMapperTests
{
    [Fact]
    public void LocalAndGrpcNodeMappings_ShouldBeEquivalent()
    {
        var createdAt = new DateTimeOffset(2026, 8, 3, 12, 0, 0, TimeSpan.Zero);
        var local = new SwarmNodeResult(
            "node-1",
            17,
            "manager-1",
            "Manager",
            true,
            "Reachable",
            "Ready",
            null,
            "Active",
            "28.0",
            "linux",
            "x86_64",
            "10.0.0.1",
            new Dictionary<string, string> { ["zone"] = "west" },
            3,
            4,
            createdAt,
            createdAt.AddMinutes(1));
        var grpc = new SwarmNodeMessage
        {
            Id = local.Id,
            VersionIndex = (ulong)local.VersionIndex,
            Hostname = local.Hostname,
            Role = local.Role,
            IsLeader = local.IsLeader,
            Reachability = local.Reachability,
            Status = local.Status,
            Availability = local.Availability,
            EngineVersion = local.EngineVersion,
            OperatingSystem = local.OperatingSystem,
            Architecture = local.Architecture,
            Address = local.Address,
            RunningTaskCount = local.RunningTaskCount,
            DesiredTaskCount = local.DesiredTaskCount,
            CreatedAt = Timestamp.FromDateTimeOffset(createdAt),
            UpdatedAt = Timestamp.FromDateTimeOffset(createdAt.AddMinutes(1))
        };
        foreach (var label in local.Labels)
            grpc.Labels.Add(label.Key, label.Value);

        var localMapped = local.Map();
        var grpcMapped = grpc.Map();

        Assert.Equal(localMapped with { Labels = grpcMapped.Labels }, grpcMapped);
        Assert.Equal("west", grpcMapped.Labels["zone"]);
    }

    [Fact]
    public void LocalAndGrpcInventoryMappings_ShouldBeEquivalent()
    {
        var timestamp = new DateTimeOffset(2026, 8, 3, 12, 0, 0, TimeSpan.Zero);
        var labels = new Dictionary<string, string> { ["stack"] = "demo" };

        var service = new SwarmServiceResult(
            "service-1", 2, "web", "Replicated", "nginx:latest", 2, 3, "Completed", null,
            ["8080:80/tcp"], ["network-1"], ["secret-1"], ["config-1"], labels, timestamp, timestamp);
        var serviceMessage = new SwarmServiceMessage
        {
            Id = service.Id,
            VersionIndex = (ulong)service.VersionIndex,
            Name = service.Name,
            Mode = service.Mode,
            Image = service.Image,
            RunningTaskCount = service.RunningTaskCount,
            DesiredTaskCount = service.DesiredTaskCount,
            UpdateState = service.UpdateState,
            Ports = { service.Ports },
            NetworkIds = { service.NetworkIds },
            SecretIds = { service.SecretIds },
            ConfigIds = { service.ConfigIds },
            CreatedAt = Timestamp.FromDateTimeOffset(timestamp),
            UpdatedAt = Timestamp.FromDateTimeOffset(timestamp)
        };
        AddLabels(serviceMessage.Labels, labels);
        Assert.Equivalent(service.Map(), serviceMessage.Map(), strict: true);

        var task = new SwarmTaskResult(
            "task-1", 3, "web.1", "service-1", 1, "node-1", "Running", "Running",
            null, null, "nginx:latest", [], timestamp, timestamp, timestamp, "container-1");
        var taskMessage = new SwarmTaskMessage
        {
            Id = task.Id,
            VersionIndex = (ulong)task.VersionIndex,
            Name = task.Name,
            ServiceId = task.ServiceId,
            Slot = task.Slot!.Value,
            NodeId = task.NodeId,
            DesiredState = task.DesiredState,
            State = task.State,
            Image = task.Image,
            ContainerId = task.ContainerId,
            StatusTimestamp = Timestamp.FromDateTimeOffset(timestamp),
            CreatedAt = Timestamp.FromDateTimeOffset(timestamp),
            UpdatedAt = Timestamp.FromDateTimeOffset(timestamp)
        };
        Assert.Equivalent(task.Map(), taskMessage.Map(), strict: true);

        var network = new SwarmNetworkResult(
            "network-1", "frontend", "swarm", "overlay", true, false, false, true, false,
            ["10.0.0.0/24"], labels, timestamp);
        var networkMessage = new SwarmNetworkMessage
        {
            Id = network.Id,
            Name = network.Name,
            Scope = network.Scope,
            Driver = network.Driver,
            IsAttachable = network.IsAttachable,
            IsInternal = network.IsInternal,
            IsIngress = network.IsIngress,
            IsEncrypted = network.IsEncrypted,
            EnableIpv6 = network.EnableIPv6,
            Subnets = { network.Subnets },
            CreatedAt = Timestamp.FromDateTimeOffset(timestamp)
        };
        AddLabels(networkMessage.Labels, labels);
        Assert.Equivalent(network.Map(), networkMessage.Map(), strict: true);

        var secret = new SwarmSecretResult("secret-1", 4, "password", null, labels, timestamp, timestamp);
        var secretMessage = new SwarmSecretMessage
        {
            Id = secret.Id,
            VersionIndex = (ulong)secret.VersionIndex,
            Name = secret.Name,
            CreatedAt = Timestamp.FromDateTimeOffset(timestamp),
            UpdatedAt = Timestamp.FromDateTimeOffset(timestamp)
        };
        AddLabels(secretMessage.Labels, labels);
        Assert.Equivalent(secret.Map(), secretMessage.Map(), strict: true);
        Assert.Null(SwarmSecretMessage.Descriptor.FindFieldByName("data"));

        var config = new SwarmConfigResult("config-1", 5, "nginx-config", "golang", labels, timestamp, timestamp);
        var configMessage = new SwarmConfigMessage
        {
            Id = config.Id,
            VersionIndex = (ulong)config.VersionIndex,
            Name = config.Name,
            TemplatingDriver = config.TemplatingDriver,
            CreatedAt = Timestamp.FromDateTimeOffset(timestamp),
            UpdatedAt = Timestamp.FromDateTimeOffset(timestamp)
        };
        AddLabels(configMessage.Labels, labels);
        Assert.Equivalent(config.Map(), configMessage.Map(), strict: true);
    }

    private static void AddLabels(
        Google.Protobuf.Collections.MapField<string, string> target,
        IReadOnlyDictionary<string, string> labels)
    {
        foreach (var label in labels)
            target.Add(label.Key, label.Value);
    }
}
