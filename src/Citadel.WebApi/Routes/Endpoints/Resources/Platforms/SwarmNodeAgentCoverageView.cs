using Domain.Contracts.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Platforms;

public sealed record SwarmNodeAgentCoverageView(
    string State,
    bool IsInstalled,
    int CoveredNodes,
    int EligibleNodes,
    int TotalNodes,
    int ConnectedNodes,
    int OfflineNodes,
    int EnrollingNodes,
    int MissingNodes,
    int IncompatibleNodes,
    int UnsupportedNodes,
    int UnschedulableNodes,
    int StaleNodes,
    DateTimeOffset? LastMembershipReconciliationAtUtc,
    string? AgentImageReference,
    string? AgentImageDigest,
    DateTimeOffset? EnrollmentExpiresAtUtc,
    SwarmNodeAgentOperationResult? Operation,
    IReadOnlyList<string> Reasons,
    IReadOnlyList<SwarmNodeAgentNodeCoverageResult> Nodes,
    bool CanManageNodeAgents)
{
    internal static SwarmNodeAgentCoverageView Map(
        SwarmNodeAgentCoverageResult value,
        bool canManageNodeAgents)
        => new(
            value.State,
            value.IsInstalled,
            value.CoveredNodes,
            value.EligibleNodes,
            value.TotalNodes,
            value.ConnectedNodes,
            value.OfflineNodes,
            value.EnrollingNodes,
            value.MissingNodes,
            value.IncompatibleNodes,
            value.UnsupportedNodes,
            value.UnschedulableNodes,
            value.StaleNodes,
            value.LastMembershipReconciliationAtUtc,
            value.AgentImageReference,
            value.AgentImageDigest,
            value.EnrollmentExpiresAtUtc,
            value.Operation,
            value.Reasons,
            value.Nodes,
            canManageNodeAgents);
}
