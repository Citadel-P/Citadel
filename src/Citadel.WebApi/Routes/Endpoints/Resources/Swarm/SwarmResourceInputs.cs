using Application.Features.Swarm.Commands;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

public sealed record UpdateSwarmNodeInput(
    long VersionIndex,
    string Availability,
    Dictionary<string, string>? Labels = null)
{
    internal UpdateSwarmNode ToCommand(Guid platformId, string nodeId) =>
        new(platformId, nodeId, VersionIndex, Availability, Labels ?? new Dictionary<string, string>());
}

public sealed record SwarmNodeAvailabilityTargetInput(string NodeId, long VersionIndex);

public sealed record UpdateSwarmNodesAvailabilityInput(
    SwarmNodeAvailabilityTargetInput[]? Nodes,
    string Availability)
{
    internal UpdateSwarmNodesAvailability ToCommand(Guid platformId) =>
        new(
            platformId,
            Nodes?.Select(node => new SwarmNodeAvailabilityTarget(node.NodeId, node.VersionIndex)).ToArray() ?? [],
            Availability);
}

public sealed record CreateSwarmSecretInput(
    string Name,
    string Data,
    Dictionary<string, string>? Labels = null)
{
    internal CreateSwarmSecret ToCommand(Guid platformId) =>
        new(platformId, Name, Data, Labels ?? new Dictionary<string, string>());
}

public sealed record CreateSwarmConfigInput(
    string Name,
    string Data,
    Dictionary<string, string>? Labels = null)
{
    internal CreateSwarmConfig ToCommand(Guid platformId) =>
        new(platformId, Name, Data, Labels ?? new Dictionary<string, string>());
}

public sealed record UpdateSwarmResourceLabelsInput(
    long VersionIndex,
    Dictionary<string, string>? Labels = null)
{
    internal UpdateSwarmSecretLabels ToSecretCommand(Guid platformId, string resourceId) =>
        new(platformId, resourceId, VersionIndex, Labels ?? new Dictionary<string, string>());

    internal UpdateSwarmConfigLabels ToConfigCommand(Guid platformId, string resourceId) =>
        new(platformId, resourceId, VersionIndex, Labels ?? new Dictionary<string, string>());
}

public sealed record DeleteSwarmResourcesInput(string[] Ids)
{
    internal DeleteSwarmServices ToServiceCommand(Guid platformId) => new(platformId, Ids);

    internal DeleteSwarmSecrets ToSecretCommand(Guid platformId) => new(platformId, Ids);

    internal DeleteSwarmConfigs ToConfigCommand(Guid platformId) => new(platformId, Ids);
}
