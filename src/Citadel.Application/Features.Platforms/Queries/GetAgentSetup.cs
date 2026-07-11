using Application.Configs;
using Application.Services;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record GetAgentSetup : IQuery<Result<AgentSetupInstructions>>;

internal sealed class GetAgentSetupHandler(
    IOptions<EdgeAgentOptions> edgeAgentOptions,
    IAgentHubPublicKeyProvider hubPublicKeyProvider)
    : IQueryHandler<GetAgentSetup, Result<AgentSetupInstructions>>
{
    public ValueTask<Result<AgentSetupInstructions>> Handle(GetAgentSetup query, CancellationToken cancellationToken)
    {
        var hubPublicKey = hubPublicKeyProvider.GetPublicKey();
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
