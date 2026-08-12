using Npgsql;
using System.Text.Json;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Compatibility;

[Collection("AcceptancePostgres")]
public sealed class SwarmNodeAgentCompatibilityTests(
    AcceptancePostgresFixture postgres)
{
    public static bool HasRequiredCandidateImages =>
        CandidateImageTestEnvironment.ShouldRun(
            "CITADEL_ACCEPTANCE_CORE_IMAGE",
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");

    [Theory(
        Skip = "Set CITADEL_ACCEPTANCE_CORE_IMAGE and CITADEL_ACCEPTANCE_AGENT_IMAGE to run the multi-node Swarm compatibility test.",
        SkipUnless = nameof(HasRequiredCandidateImages))]
    [InlineData(SwarmManagerConnectorMode.Local)]
    [InlineData(SwarmManagerConnectorMode.Agent)]
    [InlineData(SwarmManagerConnectorMode.EdgeAgent)]
    public async Task ManagerConnector_ShouldInstallRouteWorkerContainersAndRecoverPartialCoverage(
        SwarmManagerConnectorMode connectorMode)
    {
        var coreImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_CORE_IMAGE");
        var agentImage = CandidateImageTestEnvironment.GetRequiredImage(
            "CITADEL_ACCEPTANCE_AGENT_IMAGE");
        var cancellationToken = TestContext.Current.CancellationToken;
        var hostConnectionString =
            await postgres.CreateDatabaseAsync(cancellationToken);

        try
        {
            var databaseConnection =
                new NpgsqlConnectionStringBuilder(hostConnectionString);
            databaseConnection.Host =
                SwarmCompatibilityEnvironment.PostgresNetworkAlias;
            databaseConnection.Port = 5432;

            await using var environment =
                await SwarmCompatibilityEnvironment.StartAsync(
                    coreImage,
                    agentImage,
                    databaseConnection.ConnectionString,
                    postgres.ContainerId,
                    cancellationToken);
            await environment.AuthenticateAsAdminAsync(cancellationToken);
            var platformId = await environment.CreatePlatformAsync(
                connectorMode,
                cancellationToken);

            await environment.WaitForSwarmNodesAsync(platformId, 3, cancellationToken);
            await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("totalNodes").GetInt32() == 3
                    && coverage.GetProperty("eligibleNodes").GetInt32() == 3,
                "the manager and two workers to appear in Swarm coverage",
                cancellationToken);

            await environment.InstallNodeAgentsAsync(platformId, cancellationToken);
            var complete = await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("state").GetString() == "Complete"
                    && coverage.GetProperty("coveredNodes").GetInt32() == 3
                    && coverage.GetProperty("connectedNodes").GetInt32() == 3,
                "complete three-node coverage",
                cancellationToken);
            Assert.True(complete.GetProperty("isInstalled").GetBoolean());
            Assert.Equal(0, complete.GetProperty("missingNodes").GetInt32());

            await environment.DisconnectManagerConnectorAsync(
                connectorMode,
                platformId,
                cancellationToken);
            await environment.ReconnectManagerConnectorAsync(
                connectorMode,
                platformId,
                cancellationToken);
            await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("state").GetString() == "Complete"
                    && coverage.GetProperty("coveredNodes").GetInt32() == 3,
                "complete coverage after manager connector recovery",
                cancellationToken);

            var workerOne = await environment.StartWorkerContainerAsync(
                1,
                cancellationToken);
            var workerTwo = await environment.StartWorkerContainerAsync(
                2,
                cancellationToken);
            var workerOneContainer = await environment.WaitForContainerAsync(
                platformId,
                workerOne.ContainerId,
                container =>
                    container.GetProperty("dockerNodeId").GetString() == workerOne.NodeId
                    && container.GetProperty("lastStats").ValueKind == JsonValueKind.Object,
                "worker-one Container inventory and statistics",
                cancellationToken);
            var workerTwoContainer = await environment.WaitForContainerAsync(
                platformId,
                workerTwo.ContainerId,
                container =>
                    container.GetProperty("dockerNodeId").GetString() == workerTwo.NodeId
                    && container.GetProperty("lastStats").ValueKind == JsonValueKind.Object,
                "worker-two Container inventory and statistics",
                cancellationToken);

            await environment.AssertContainerInspectRoutesAsync(
                workerOneContainer,
                cancellationToken);
            await environment.AssertContainerInspectRoutesAsync(
                workerTwoContainer,
                cancellationToken);
            await environment.AssertNodeLocalResourcesRouteAsync(
                platformId,
                1,
                workerOne.NodeId,
                cancellationToken);
            await environment.AssertNodeLocalResourcesRouteAsync(
                platformId,
                2,
                workerTwo.NodeId,
                cancellationToken);

            await environment.StopWorkerTwoAsync(cancellationToken);
            var partial = await environment.WaitForCoverageAsync(
                platformId,
                coverage =>
                    coverage.GetProperty("state").GetString() == "Partial"
                    && coverage.GetProperty("eligibleNodes").GetInt32() == 3
                    && coverage.GetProperty("coveredNodes").GetInt32() == 2
                    && coverage.GetProperty("nodes").EnumerateArray().Any(
                        node => node.GetProperty("dockerNodeId").GetString() == workerTwo.NodeId
                                && node.GetProperty("agentConnectionState").GetString() == "Offline"),
                "partial coverage after worker two disconnects",
                cancellationToken);
            Assert.Equal(1, partial.GetProperty("offlineNodes").GetInt32());

            await environment.WaitForContainerAsync(
                platformId,
                workerTwo.ContainerId,
                static container =>
                    container.GetProperty("projectionStaleSince").ValueKind == JsonValueKind.Number,
                "worker-two Container projection to be retained as stale",
                cancellationToken);
            await environment.WaitForNodeVolumeStateAsync(
                platformId,
                workerTwo.NodeId,
                isStale: true,
                cancellationToken);

            await environment.RestartWorkerTwoAsync(cancellationToken);
            await environment.WaitForCoverageAsync(
                platformId,
                static coverage =>
                    coverage.GetProperty("state").GetString() == "Complete"
                    && coverage.GetProperty("coveredNodes").GetInt32() == 3
                    && coverage.GetProperty("offlineNodes").GetInt32() == 0,
                "complete coverage after worker two reconnects",
                cancellationToken);
            await environment.WaitForContainerAsync(
                platformId,
                workerTwo.ContainerId,
                static container =>
                    container.GetProperty("projectionStaleSince").ValueKind == JsonValueKind.Null
                    && container.GetProperty("lastStats").ValueKind == JsonValueKind.Object,
                "worker-two Container projection and statistics to recover",
                cancellationToken);
            await environment.WaitForNodeVolumeStateAsync(
                platformId,
                workerTwo.NodeId,
                isStale: false,
                cancellationToken);
        }
        finally
        {
            await postgres.DropDatabaseAsync(
                hostConnectionString,
                CancellationToken.None);
        }
    }
}
