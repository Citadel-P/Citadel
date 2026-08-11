using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

internal interface ISwarmManagerIdentityValidator
{
    Task<Result> ValidateAsync(Platform platform, CancellationToken cancellationToken);
}

internal sealed class SwarmManagerIdentityValidator(
    IConnectorFactory<IPlatformConnector> connectorFactory) : ISwarmManagerIdentityValidator
{
    public async Task<Result> ValidateAsync(Platform platform, CancellationToken cancellationToken)
    {
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor expected
            || string.IsNullOrWhiteSpace(platform.ClusterId)
            || string.IsNullOrWhiteSpace(expected.NodeID)
            || string.IsNullOrWhiteSpace(expected.DaemonId))
        {
            return Result.Failure(new ConflictError(
                "ManagerIdentityChanged: the persisted Swarm manager identity is incomplete."));
        }

        var observed = await connectorFactory.GetConnector(platform.ConnectorType).GetPlatformAsync(
            new GetPlatformCommand(platform.Address, platform.Name),
            cancellationToken);
        if (!observed.IsSuccess(out var current, out var error))
        {
            return Result.Failure(new ServiceUnavailableError(
                $"ManagerUnavailable: {error?.Message ?? "the Swarm manager could not be validated."}"));
        }

        if (current.Descriptor is not DockerSwarmPlatformDescriptor actual
            || !actual.ControlAvailable
            || !string.Equals(platform.ClusterId, actual.ClusterId, StringComparison.Ordinal)
            || !string.Equals(expected.NodeID, actual.NodeID, StringComparison.Ordinal)
            || !string.Equals(expected.DaemonId, actual.DaemonId, StringComparison.Ordinal))
        {
            return Result.Failure(new ConflictError(
                "ManagerIdentityChanged: the endpoint no longer resolves to the pinned Swarm manager."));
        }

        return Result.Success();
    }
}
