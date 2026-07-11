using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record RotateAgentHubKey : ICommand<Result<AgentSetupInstructions>>;

internal sealed class RotateAgentHubKeyHandler(
    IOptions<EdgeAgentOptions> edgeAgentOptions,
    IAgentHubPublicKeyProvider hubPublicKeyProvider)
    : ICommandHandler<RotateAgentHubKey, Result<AgentSetupInstructions>>
{
    public ValueTask<Result<AgentSetupInstructions>> Handle(RotateAgentHubKey command, CancellationToken cancellationToken)
    {
        var hubPublicKey = hubPublicKeyProvider.RotateKeyPair();
        var environment = new Dictionary<string, string>
        {
            ["HUB_PUBLIC_KEY"] = hubPublicKey
        };
        var agentImage = edgeAgentOptions.Value.GetAgentImage();
        var dockerRunCommand = AgentDockerCommandBuilder.BuildRegularAgentCommand(agentImage, environment);

        return ValueTask.FromResult(Result.Success(new AgentSetupInstructions(
            hubPublicKey,
            environment,
            agentImage,
            dockerRunCommand)));
    }
}
