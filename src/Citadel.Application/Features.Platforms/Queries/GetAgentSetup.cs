using Application.Configs;
using Application.Services;
using Domain.Configs;
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
    IOptions<AgentTransportOptions> agentTransportOptions,
    IAgentHubPublicKeyProvider hubPublicKeyProvider)
    : IQueryHandler<GetAgentSetup, Result<AgentSetupInstructions>>
{
    public ValueTask<Result<AgentSetupInstructions>> Handle(GetAgentSetup query, CancellationToken cancellationToken)
    {
        var hubPublicKey = hubPublicKeyProvider.GetPublicKey();
        var requiresTls = !agentTransportOptions.Value.AllowInsecure;
        var environment =
            AgentDockerCommandBuilder.BuildRegularAgentEnvironment(
                hubPublicKey,
                requiresTls);
        var agentImage = edgeAgentOptions.Value.GetAgentImage();
        var dockerRunCommand = AgentDockerCommandBuilder.BuildRegularAgentCommand(
            agentImage,
            environment,
            requiresTls);

        return ValueTask.FromResult(Result.Success(new AgentSetupInstructions(
            hubPublicKey,
            environment,
            agentImage,
            dockerRunCommand,
            RequiresTls: requiresTls)));
    }
}
