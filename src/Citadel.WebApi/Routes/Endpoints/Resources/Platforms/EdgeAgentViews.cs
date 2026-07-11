using Domain.Contracts.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record EdgeAgentEnrollmentView(
    Guid EnrollmentId,
    Guid PlatformId,
    string Token,
    DateTime ExpiresAtUtc,
    EdgeAgentEnrollmentInstructionsView Instructions)
{
    internal static EdgeAgentEnrollmentView Map(EdgeAgentEnrollmentResult result)
        => new(
            result.EnrollmentId,
            result.PlatformId,
            result.Token,
            result.ExpiresAtUtc,
            EdgeAgentEnrollmentInstructionsView.Map(result.Instructions));
}

public sealed record EdgeAgentEnrollmentInstructionsView(
    string CoreUrl,
    IReadOnlyDictionary<string, string> Environment,
    string AgentImage,
    string DockerRunCommand)
{
    internal static EdgeAgentEnrollmentInstructionsView Map(EdgeAgentEnrollmentInstructions instructions)
        => new(
            instructions.CoreUrl,
            instructions.Environment,
            instructions.AgentImage,
            instructions.DockerRunCommand);
}

public sealed record EdgeAgentStatusView(
    string ConnectionStatus,
    DateTime? LastConnectedAtUtc,
    DateTime? LastDisconnectedAtUtc,
    DateTime? LastHeartbeatAtUtc,
    string? LastSeenVersion,
    string? LastSeenHostname,
    string? AgentFingerprint,
    int? ProtocolVersion,
    DateTime? RevokedAtUtc,
    DateTime? EnrollmentExpiresAtUtc)
{
    internal static EdgeAgentStatusView Map(EdgeAgentStatusResult result)
        => new(
            result.ConnectionStatus,
            result.LastConnectedAtUtc,
            result.LastDisconnectedAtUtc,
            result.LastHeartbeatAtUtc,
            result.LastSeenVersion,
            result.LastSeenHostname,
            result.AgentFingerprint,
            result.ProtocolVersion,
            result.RevokedAtUtc,
            result.EnrollmentExpiresAtUtc);
}
