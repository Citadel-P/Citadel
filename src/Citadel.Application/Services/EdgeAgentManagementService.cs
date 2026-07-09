using System.Security.Cryptography;
using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services;

internal sealed class EdgeAgentManagementService(IServiceScopeFactory scopeFactory) : IEdgeAgentManagementService
{
    public async Task<Result<EdgeAgentEnrollmentResult>> CreateEnrollmentAsync(Guid platformId, string coreUrl, Guid actorId, TimeSpan ttl, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<EdgeAgentEnrollmentResult>(new NotFoundError("Platform not found."));
        }

        if (platform.ConnectorType != PlatformConnectorType.EdgeAgent)
        {
            return Result.Failure<EdgeAgentEnrollmentResult>(new BadRequestError("Platform is not an Edge Agent platform."));
        }

        var binding = await unitOfWork.EdgeAgents.GetBindingByPlatformIdAsync(platformId, cancellationToken);
        if (binding?.IsRevoked == true)
        {
            return Result.Failure<EdgeAgentEnrollmentResult>(new ConflictError("Edge Agent binding is revoked."));
        }

        var utcNow = DateTime.UtcNow;
        var token = GenerateToken();
        var expiresAt = utcNow.Add(ttl);
        var enrollment = new EdgeAgentEnrollment(
            Id: Guid.CreateVersion7(),
            PlatformId: platformId,
            TokenHash: HashToken(token),
            ExpiresAtUtc: expiresAt,
            UsedAtUtc: null,
            RevokedAtUtc: null,
            CreatedByActorId: actorId,
            CreatedAtUtc: utcNow);

        await unitOfWork.EdgeAgents.AddEnrollmentAsync(enrollment, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var environment = new Dictionary<string, string>
        {
            ["CITADEL_AGENT_MODE"] = "edge",
            ["CITADEL_CORE_URL"] = coreUrl,
            ["CITADEL_EDGE_ENROLLMENT_TOKEN"] = token,
            ["CITADEL_EDGE_AGENT_KEY_PATH"] = "/app/data/edge-agent.key",
            ["CITADEL_EDGE_IDENTITY_PATH"] = "/app/data/edge-agent.identity.json"
        };

        return Result.Success(new EdgeAgentEnrollmentResult(
            enrollment.Id,
            platformId,
            token,
            expiresAt,
            new EdgeAgentEnrollmentInstructions(coreUrl, environment)));
    }

    public async Task<Result<EdgeAgentStatusResult>> GetStatusAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<EdgeAgentStatusResult>(new NotFoundError("Platform not found."));
        }

        if (platform.ConnectorType != PlatformConnectorType.EdgeAgent)
        {
            return Result.Failure<EdgeAgentStatusResult>(new BadRequestError("Platform is not an Edge Agent platform."));
        }

        var binding = await unitOfWork.EdgeAgents.GetBindingByPlatformIdAsync(platformId, cancellationToken);
        var activeEnrollment = await unitOfWork.EdgeAgents.GetActiveEnrollmentAsync(platformId, utcNow, cancellationToken);
        if (binding is null)
        {
            return Result.Success(new EdgeAgentStatusResult(
                "PendingEnrollment",
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                activeEnrollment?.ExpiresAtUtc));
        }

        var status = binding.IsRevoked
            ? "Revoked"
            : binding.ConnectionStatus.ToString();

        return Result.Success(new EdgeAgentStatusResult(
            status,
            binding.LastConnectedAtUtc,
            binding.LastDisconnectedAtUtc,
            binding.LastHeartbeatAtUtc,
            binding.LastSeenVersion,
            binding.LastSeenHostname,
            ShortFingerprint(binding.AgentFingerprint),
            binding.ProtocolVersion,
            binding.RevokedAtUtc,
            activeEnrollment?.ExpiresAtUtc));
    }

    public async Task<Result<EdgeAgentEnrollmentCompleteResult>> CompleteEnrollmentAsync(EdgeAgentEnrollmentRequest request, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var enrollment = await unitOfWork.EdgeAgents.GetEnrollmentByTokenHashAsync(HashToken(request.EnrollmentToken), cancellationToken);
        if (enrollment is null || !enrollment.IsActive(utcNow))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new UnauthorizedError("Enrollment token is invalid, expired, revoked, or already used."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(enrollment.PlatformId, cancellationToken);
        if (platform is null || platform.ConnectorType != PlatformConnectorType.EdgeAgent)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new BadRequestError("Enrollment token is not scoped to a valid Edge Agent platform."));
        }

        var existingBinding = await unitOfWork.EdgeAgents.GetBindingByPlatformIdAsync(enrollment.PlatformId, cancellationToken);
        if (existingBinding is not null && !existingBinding.IsRevoked)
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new ConflictError("Edge Agent platform is already enrolled."));
        }

        var publicKeyBytes = Convert.FromBase64String(request.AgentPublicKey);
        var fingerprint = GetFingerprint(publicKeyBytes);
        if (!string.IsNullOrWhiteSpace(request.AgentFingerprint) &&
            !string.Equals(request.AgentFingerprint, fingerprint, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentEnrollmentCompleteResult>(new BadRequestError("Agent fingerprint does not match the provided public key."));
        }

        var agentId = Guid.CreateVersion7();
        var now = utcNow;
        var binding = new EdgeAgentBinding(
            Id: Guid.CreateVersion7(),
            PlatformId: enrollment.PlatformId,
            AgentId: agentId,
            AgentPublicKey: request.AgentPublicKey,
            AgentFingerprint: fingerprint,
            ConnectionStatus: EdgeAgentConnectionStatus.Offline,
            LastConnectedAtUtc: null,
            LastDisconnectedAtUtc: null,
            LastHeartbeatAtUtc: null,
            LastSeenVersion: request.AgentVersion,
            LastSeenHostname: request.Hostname,
            CapabilitiesJson: string.IsNullOrWhiteSpace(request.CapabilitiesJson) ? "{}" : request.CapabilitiesJson,
            ProtocolVersion: request.ProtocolVersion,
            RevokedAtUtc: null,
            CreatedAtUtc: now,
            UpdatedAtUtc: now);

        await unitOfWork.EdgeAgents.AddBindingAsync(binding, cancellationToken);
        await unitOfWork.EdgeAgents.MarkEnrollmentUsedAsync(enrollment.Id, now, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new EdgeAgentEnrollmentCompleteResult(enrollment.PlatformId, agentId));
    }

    public async Task<Result<EdgeAgentBinding>> GetReconnectBindingAsync(Guid platformId, Guid agentId, string agentFingerprint, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var binding = await unitOfWork.EdgeAgents.GetBindingByAgentAsync(platformId, agentId, cancellationToken);
        if (binding is null)
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent binding was not found."));
        }

        if (binding.IsRevoked)
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent binding is revoked."));
        }

        if (!string.Equals(binding.AgentFingerprint, agentFingerprint, StringComparison.Ordinal))
        {
            return Result.Failure<EdgeAgentBinding>(new UnauthorizedError("Edge Agent fingerprint does not match."));
        }

        return Result.Success(binding);
    }

    public async Task MarkConnectedAsync(Guid platformId, string hostname, string agentVersion, string capabilitiesJson, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateBindingConnectedAsync(platformId, utcNow, hostname, agentVersion, capabilitiesJson, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
    }

    public async Task MarkHeartbeatAsync(Guid platformId, EdgeAgentHeartbeatSnapshot heartbeat, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateBindingHeartbeatAsync(platformId, utcNow, heartbeat.Hostname, heartbeat.AgentVersion, heartbeat.CapabilitiesJson, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
    }

    public async Task MarkDisconnectedAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.EdgeAgents.UpdateBindingDisconnectedAsync(platformId, utcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
    }

    public async Task<Result> RevokeAsync(Guid platformId, DateTime utcNow, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure(new NotFoundError("Platform not found."));
        }

        if (platform.ConnectorType != PlatformConnectorType.EdgeAgent)
        {
            return Result.Failure(new BadRequestError("Platform is not an Edge Agent platform."));
        }

        await unitOfWork.EdgeAgents.RevokeBindingAsync(platformId, utcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }

    public static string HashToken(string token)
    {
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(token));
        return Base64Url(hash);
    }

    public static string GetFingerprint(ReadOnlySpan<byte> publicKey)
    {
        var hash = SHA256.HashData(publicKey);
        return $"SHA256:{Convert.ToHexString(hash).ToLowerInvariant()}";
    }

    public static string ShortFingerprint(string fingerprint)
    {
        const int prefixLength = 7;
        if (!fingerprint.StartsWith("SHA256:", StringComparison.Ordinal) || fingerprint.Length <= prefixLength + 16)
        {
            return fingerprint;
        }

        var value = fingerprint[prefixLength..];
        return $"SHA256:{value[..8]}...{value[^4..]}";
    }

    private static string GenerateToken()
    {
        Span<byte> bytes = stackalloc byte[32];
        RandomNumberGenerator.Fill(bytes);
        return Base64Url(bytes);
    }

    private static string Base64Url(ReadOnlySpan<byte> bytes)
    {
        return Convert.ToBase64String(bytes)
            .TrimEnd('=')
            .Replace('+', '-')
            .Replace('/', '_');
    }
}
