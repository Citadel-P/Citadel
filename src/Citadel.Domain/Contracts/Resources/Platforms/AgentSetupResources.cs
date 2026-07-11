namespace Domain.Contracts.Resources.Platforms;

public sealed record AgentSetupInstructions(
    string HubPublicKey,
    IReadOnlyDictionary<string, string> Environment,
    string AgentImage,
    string DockerRunCommand);
