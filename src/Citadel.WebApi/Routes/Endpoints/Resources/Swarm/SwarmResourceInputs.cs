using Application.Features.Swarm.Commands;

namespace WebApi.Routes.Endpoints.Resources.Swarm;

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
    internal DeleteSwarmSecrets ToSecretCommand(Guid platformId) => new(platformId, Ids);

    internal DeleteSwarmConfigs ToConfigCommand(Guid platformId) => new(platformId, Ids);
}
