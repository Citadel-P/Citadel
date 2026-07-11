using Hosting.Common;

namespace Application.Services;

public interface IAgentHubPublicKeyProvider
{
    string GetPublicKey();
    string RotateKeyPair();
}

internal sealed class AgentHubPublicKeyProvider : IAgentHubPublicKeyProvider
{
    public string GetPublicKey() => Helpers.GetOrCreatePublicKey();

    public string RotateKeyPair() => Helpers.RotateHubKeyPair();
}
