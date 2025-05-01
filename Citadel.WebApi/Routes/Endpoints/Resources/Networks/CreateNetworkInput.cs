using Application.Features.Networks.Commands;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record CreateNetworkInput(
    Guid PlatformId,
    string Name,
    string Driver,
    string Scope,
    bool? Internal,
    bool? Attachable,
    bool? Ingress,
    bool? EnableIPv6,
    bool? EnableIPv4,
    bool? ConfigOnly,
    IPAMInput IPAM = null,
    ConfigFromInput ConfigFrom = null,
    Dictionary<string, string> Labels = null,
    Dictionary<string, string> Options = null
    )
{
    internal CreateNetwork ToCommand() =>
        new (
            PlatformId,
            Name,
            Driver,
            Scope,
            Internal,
            Attachable,
            Ingress,
            EnableIPv6,
            EnableIPv4,
            ConfigOnly,
            IPAM?.ToCommand(),
            ConfigFrom?.ToCommand(),
            Labels,
            Options
        );
};

public sealed record IPAMInput(
    string Driver,
    List<IPAMConfigInput> Configs = null,
    Dictionary<string, string> Options = null
    )
{
    internal IPAM ToCommand() =>
        new (
            Driver,
            Configs?.Select(c => c.ToCommand()).ToList(),
            Options
        );
};

public sealed record IPAMConfigInput(string Subnet, string IpRange, string Gateway)
{
    internal IPAMConfig ToCommand() => new (Subnet, IpRange, Gateway);
}
public sealed record ConfigFromInput(string Network)
{
    internal ConfigFrom ToCommand() => new (Network);
}
