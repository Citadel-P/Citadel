using Domain.Contracts.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record AgentSetupView(
    string HubPublicKey,
    IReadOnlyDictionary<string, string> Environment,
    string AgentImage,
    string DockerRunCommand,
    bool RequiresTls)
{
    internal static AgentSetupView Map(AgentSetupInstructions instructions)
        => new(
            instructions.HubPublicKey,
            instructions.Environment,
            instructions.AgentImage,
            instructions.DockerRunCommand,
            instructions.RequiresTls);
}
