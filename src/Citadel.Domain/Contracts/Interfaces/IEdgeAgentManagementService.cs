using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IEdgeAgentManagementService
{
    Task<Result<EdgeAgentEnrollmentResult>> CreateEnrollmentAsync(Guid platformId, string coreUrl, Guid actorId, TimeSpan ttl, CancellationToken cancellationToken);
    Task<Result<EdgeAgentEnrollmentResult>> CreateBuildAgentPoolEnrollmentAsync(Guid buildAgentPoolId, string coreUrl, Guid actorId, TimeSpan ttl, CancellationToken cancellationToken);
    Task<Result<EdgeAgentStatusResult>> GetStatusAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken);
    Task<Result<EdgeAgentStatusResult>> GetBuildAgentPoolStatusAsync(Guid buildAgentPoolId, DateTime utcNow, CancellationToken cancellationToken);
    Task<Result<EdgeAgentEnrollmentCompleteResult>> CompleteEnrollmentAsync(EdgeAgentEnrollmentRequest request, DateTime utcNow, CancellationToken cancellationToken);
    Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(Guid platformId, Guid agentId, string agentFingerprint, string daemonId, CancellationToken cancellationToken);
    Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(EdgeAgentResourceType resourceType, Guid resourceId, Guid agentId, string agentFingerprint, string daemonId, CancellationToken cancellationToken);
    Task MarkConnectedAsync(Guid platformId, string hostname, string agentVersion, string capabilitiesJson, DateTime utcNow, CancellationToken cancellationToken);
    Task MarkConnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, string hostname, string agentVersion, string capabilitiesJson, DateTime utcNow, CancellationToken cancellationToken);
    Task MarkHeartbeatAsync(Guid platformId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken);
    Task MarkHeartbeatAsync(EdgeAgentResourceType resourceType, Guid resourceId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken);
    Task MarkDisconnectedAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken);
    Task MarkDisconnectedAsync(EdgeAgentResourceType resourceType, Guid resourceId, DateTime utcNow, CancellationToken cancellationToken);
    Task<Result> RevokeAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken);
    Task<Result> RevokeBuildAgentPoolAsync(Guid buildAgentPoolId, DateTime utcNow, CancellationToken cancellationToken);
}
