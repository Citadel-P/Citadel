namespace Domain.Contracts.Resources.Platforms;

public sealed record SwarmNodeAgentCoverageResult(
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
    IReadOnlyList<SwarmNodeAgentNodeCoverageResult> Nodes);

public sealed record SwarmNodeAgentOperationResult(
    Guid OperationId,
    string Kind,
    string State,
    DateTime StartedAtUtc,
    string? Error);

public sealed record SwarmNodeAgentNodeCoverageResult(
    string DockerNodeId,
    string Hostname,
    string Role,
    string Availability,
    string NodeStatus,
    string Architecture,
    string DataSource,
    bool Eligible,
    bool Supported,
    bool Schedulable,
    string? ServiceTaskState,
    string AgentConnectionState,
    bool DockerReachable,
    bool Compatible,
    bool ProjectionStale,
    DateTime? LastHeartbeatAtUtc,
    DateTimeOffset? LastSuccessfulReconciliationAt,
    DateTimeOffset? StaleSince,
    string? StaleReason,
    IReadOnlyList<string> Reasons);

public sealed record SwarmNodeAgentProgressItem(
    Guid PlatformId,
    Guid OperationId,
    string Stage,
    string Message,
    bool IsCompleted = false,
    bool IsWarning = false,
    string? ErrorMessage = null);
