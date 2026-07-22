using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Builds;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services.Builds;

public interface IBuildAgentPoolValidationService
{
    Task<(BuildAgentPoolValidationStatus Status, string Message)> ValidateAsync(BuildAgentPool pool, CancellationToken cancellationToken);
}

internal sealed class BuildAgentPoolValidationService(IConnectorFactory<IImageConnector> imageConnectorFactory)
    : IBuildAgentPoolValidationService
{
    public async Task<(BuildAgentPoolValidationStatus Status, string Message)> ValidateAsync(
        BuildAgentPool pool,
        CancellationToken cancellationToken)
    {
        if (pool.ProviderSpec is not SelfManagedVmBuildAgentPoolProviderSpec vm)
            return (BuildAgentPoolValidationStatus.Invalid, "Only self-managed Citadel Agent build pools can be tested.");

        var target = ResolveSelfManagedVmTarget(pool, vm);
        if (!target.IsSuccess(out var checkTarget, out var targetError))
            return (BuildAgentPoolValidationStatus.Invalid, targetError?.Message ?? "Build pool target is invalid.");

        return await TestSelfManagedVmAsync(checkTarget.Address, checkTarget.ConnectorType, cancellationToken);
    }

    private static Result<SelfManagedVmCheckTarget> ResolveSelfManagedVmTarget(
        BuildAgentPool pool,
        SelfManagedVmBuildAgentPoolProviderSpec vm)
    {
        if (vm.ConnectionMode == BuildAgentPoolConnectionMode.EdgeAgent)
            return new SelfManagedVmCheckTarget($"edge-build-pool://{pool.Id:D}", PlatformConnectorType.EdgeAgent);

        if (string.IsNullOrWhiteSpace(vm.Endpoint))
            return Result.Failure<SelfManagedVmCheckTarget>(new BadRequestError("Self-managed VM endpoint is required for this build pool."));

        return new SelfManagedVmCheckTarget(vm.Endpoint, PlatformConnectorType.Agent);
    }

    private async Task<(BuildAgentPoolValidationStatus Status, string Message)> TestSelfManagedVmAsync(
        string endpoint,
        PlatformConnectorType connectorType,
        CancellationToken cancellationToken)
    {
        try
        {
            var connector = imageConnectorFactory.GetConnector(connectorType);
            var result = await connector.CheckBuildHostAsync(endpoint, cancellationToken);
            if (!result.IsSuccess(out var capabilities, out var error))
                return (BuildAgentPoolValidationStatus.Invalid, $"Citadel Agent build capability check failed: {error?.Message ?? "Unknown error."}");

            if (!capabilities.Available)
                return (BuildAgentPoolValidationStatus.Invalid, "Citadel Agent can be reached, but Docker build capabilities are not available.");

            return (BuildAgentPoolValidationStatus.Ready, BuildCapabilityMessage(capabilities));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            return (BuildAgentPoolValidationStatus.Invalid, $"Citadel Agent build capability check failed: {ex.Message}");
        }
    }

    private sealed record SelfManagedVmCheckTarget(string Address, PlatformConnectorType ConnectorType);

    private static string BuildCapabilityMessage(BuildHostCapabilitiesResult capabilities)
    {
        var docker = string.IsNullOrWhiteSpace(capabilities.DockerVersion) ? "Docker" : $"Docker {capabilities.DockerVersion}";
        var api = string.IsNullOrWhiteSpace(capabilities.ApiVersion) ? null : $"API {capabilities.ApiVersion}";
        var platform = string.Join(
            "/",
            new[] { capabilities.OperatingSystem, capabilities.Architecture }.Where(static value => !string.IsNullOrWhiteSpace(value)));
        var buildKit = string.IsNullOrWhiteSpace(capabilities.BuildKitVersion)
            ? null
            : $"BuildKit {capabilities.BuildKitVersion}";

        return string.Join(" - ", new[] { docker, api, string.IsNullOrWhiteSpace(platform) ? null : platform, buildKit }.Where(static value => !string.IsNullOrWhiteSpace(value)));
    }
}
