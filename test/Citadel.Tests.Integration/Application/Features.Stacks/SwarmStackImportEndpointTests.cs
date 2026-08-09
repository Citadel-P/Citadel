using System.Collections.Immutable;
using System.Net;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Infrastructure.Repositories.DbQueue;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public sealed class SwarmStackImportEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<ISwarmConnector> connector = new();
    private readonly Mock<IConnectorFactory<ISwarmConnector>> connectorFactory = new();
    private readonly Mock<IContainerConnector> containerConnector = new();
    private readonly Mock<IConnectorFactory<IContainerConnector>> containerConnectorFactory = new();
    private readonly Mock<IStackConnector> stackConnector = new();
    private readonly Mock<IConnectorFactory<IStackConnector>> stackConnectorFactory = new();
    private readonly Mock<ISwarmReconciliationCoordinator> reconciliationCoordinator = new();
    private Guid platformId;
    private Guid importedStackId;
    private Guid importedReleaseId;
    private bool stackDeployDispatched;
    private string deployedServiceName = "sample_web";
    private string deployedStackNamespace = "sample";
    private StackApplyCommand? dispatchedCommand;
    private const string TaskContainerId = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    private const string ComposeContainerId = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.RemoveAll<IDbWorkQueue>();
        services.RemoveAll<IConnectorFactory<ISwarmConnector>>();
        services.RemoveAll<IConnectorFactory<IContainerConnector>>();
        services.RemoveAll<IConnectorFactory<IStackConnector>>();
        services.RemoveAll<ISwarmReconciliationCoordinator>();
        services.AddHostedService<DbWriteWorker>();
        services.AddSingleton<IDbWorkQueue, DbWorkQueue>();
        services.AddSingleton(connectorFactory.Object);
        services.AddSingleton(containerConnectorFactory.Object);
        services.AddSingleton(stackConnectorFactory.Object);
        services.AddSingleton(reconciliationCoordinator.Object);
        connectorFactory.Setup(value => value.GetConnector(PlatformConnectorType.Agent)).Returns(connector.Object);
        containerConnectorFactory.Setup(value => value.GetConnector(PlatformConnectorType.Agent)).Returns(containerConnector.Object);
        stackConnectorFactory.Setup(value => value.GetConnector(PlatformConnectorType.Agent)).Returns(stackConnector.Object);
        stackConnector
            .Setup(value => value.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Returns((StackApplyCommand command, CancellationToken cancellationToken) =>
            {
                dispatchedCommand = command;
                return SuccessfulApplyStream(command, cancellationToken);
            });
        reconciliationCoordinator
            .Setup(value => value.RefreshAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var platform = new Platform(
            "import-manager",
            "https://swarm-import.example.test",
            0,
            0,
            0,
            4,
            1024,
            "28.0.0",
            "1.0.0",
            PlatformStatus.Online,
            PlatformConnectorType.Agent,
            new DockerSwarmPlatformDescriptor(
                "node-1", "10.0.0.1", "Active", true, 1, 1,
                "daemon-1", 0, 0, 0, 0, "cluster-import"),
            clusterId: "cluster-import");
        platformId = platform.Id;
        await uow.Platforms.AddAsync(platform, cancellationToken);
        await uow.Containers.AddAsync(
            new Container(
                "/sample_web.1.task-id",
                "sha256:nginx",
                platform.Id,
                TaskContainerId,
                ContainerStateStatus.Running,
                dockerStack: "sample",
                isSwarmTask: true),
            cancellationToken);
        await uow.Containers.AddAsync(
            new Container(
                "/composeapp-web-1",
                "sha256:nginx",
                platform.Id,
                ComposeContainerId,
                ContainerStateStatus.Running,
                dockerStack: "composeapp"),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        Services.GetRequiredService<IPlatformContainerCache>().ReplacePlatformContainers(
            platform.Id,
            new PlatformCacheEntry(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                ImmutableDictionary<string, Guid>.Empty));

        var service = CreateService();
        await uow.Swarm.ReplaceAsync(
            platform.Id,
            new SwarmProjectionSnapshot(
                [],
                [SwarmServiceProjection.FromObservation(platform.Id, service, DateTimeOffset.UtcNow)],
                [],
                [],
                [],
                []),
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        connector.Setup(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => Result.Success<IReadOnlyList<SwarmServiceResult>>(
                [stackDeployDispatched ? CreateOwnedService() : service]));
        connector.Setup(value => value.ListNetworksAsync(It.IsAny<ListSwarmNetworksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmNetworkResult>>([]));
        connector.Setup(value => value.ListTasksAsync(It.IsAny<ListSwarmTasksCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmTaskResult>>([]));
        connector.Setup(value => value.ListSecretsAsync(It.IsAny<ListSwarmSecretsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmSecretResult>>([]));
        connector.Setup(value => value.ListConfigsAsync(It.IsAny<ListSwarmConfigsCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyList<SwarmConfigResult>>([]));
        containerConnector
            .Setup(value => value.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>
                {
                    [ComposeContainerId] = new(
                        "/composeapp-web-1",
                        "nginx:latest",
                        ComposeContainerId,
                        "sha256:nginx",
                        ContainerStateStatus.Running,
                        Stack: "composeapp")
                }));
        containerConnector
            .Setup(value => value.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((InspectContainerCommand command, CancellationToken _) =>
                Result.Success(CreateComposeInspection(command.ContainerId)));
    }

    [Fact]
    public async Task ComposeProjectOnSwarm_ShouldImportWithoutMutationAndConvertOnFirstApply()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var draftResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/composeapp?importKind=ComposeProject",
            cancellationToken);
        var draftBody = await draftResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(draftResponse.IsSuccessStatusCode, draftBody);
        using (var draft = JsonDocument.Parse(draftBody))
            Assert.Equal("ComposeProject", draft.RootElement.GetProperty("importKind").GetString());

        const string compose = "services:\n  web:\n    image: nginx:latest\n    environment:\n      TOKEN: ${IMPORT_TOKEN}\n    deploy:\n      replicas: 1\n";
        var validateJson = $$"""
            {
              "name": "composeapp",
              "stackSource": "WebEditor",
              "importKind": "ComposeProject",
              "spec": {
                "$type": "WebEditor",
                "composeFile": {{JsonSerializer.Serialize(compose)}},
                "updateBehavior": "Disabled",
                "projectName": "composeapp",
                "destroyBeforeDeploy": false
              }
            }
            """;
        using var validateResponse = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/composeapp/import-draft",
            new StringContent(validateJson, Encoding.UTF8, "application/json"),
            cancellationToken);
        var validateBody = await validateResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(validateResponse.IsSuccessStatusCode, validateBody);
        Assert.DoesNotContain("runtime-token-value", validateBody, StringComparison.Ordinal);
        using var validation = JsonDocument.Parse(validateBody);
        var fingerprint = validation.RootElement.GetProperty("previewFingerprint").GetString();
        Assert.True(validation.RootElement.GetProperty("canImportSensitiveEnvironmentValues").GetBoolean());
        Assert.Equal(
            "IMPORT_TOKEN",
            Assert.Single(validation.RootElement.GetProperty("importableSensitiveEnvironmentNames").EnumerateArray())
                .GetString());

        var importJson = $$"""
            {
              "name": "composeapp",
              "description": "Imported Compose project",
              "stackSource": "WebEditor",
              "importKind": "ComposeProject",
              "spec": {
                "$type": "WebEditor",
                "composeFile": {{JsonSerializer.Serialize(compose)}},
                "updateBehavior": "Disabled",
                "projectName": "composeapp",
                "destroyBeforeDeploy": false
              },
              "previewFingerprint": "{{fingerprint}}",
              "importSensitiveEnvironmentAsSecrets": true
            }
            """;
        using var importResponse = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/composeapp/import",
            new StringContent(importJson, Encoding.UTF8, "application/json"),
            cancellationToken);
        var importBody = await importResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(importResponse.IsSuccessStatusCode, importBody);
        using var imported = JsonDocument.Parse(importBody);
        importedStackId = imported.RootElement.GetProperty("id").GetGuid();

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var container = await uow.Containers.GetByIdAsync(ComposeContainerId, cancellationToken);
            Assert.Equal(importedStackId, container?.StackId);
            Assert.False(container?.IsSwarmTask);
            Assert.Equal("composeapp", (await uow.Stacks.GetSwarmNamespaceReservationAsync(importedStackId, cancellationToken))?.Namespace);

            var binding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
                ResourceBindingScope.Stack,
                importedStackId,
                cancellationToken));
            Assert.Equal("IMPORT_TOKEN", binding.Name);
            Assert.Equal(ResourceBindingKind.Secret, binding.Kind);
            Assert.Equal(SecretDeliveryMode.EnvironmentVariable, binding.SecretDeliveryMode);
            var storedValue = await uow.SecretDefinitions.GetInternalValueAsync(binding.SecretId!.Value, cancellationToken);
            Assert.NotNull(storedValue);
            Assert.NotEqual("runtime-token-value", storedValue.EncryptedValue);
            Assert.Equal(
                "runtime-token-value",
                Services.GetRequiredService<ISecretValueProtector>().Unprotect(storedValue.EncryptedValue));
        }
        stackConnector.Verify(
            value => value.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        deployedStackNamespace = "composeapp";
        using var applyResponse = await Client.PostAsync(
            "/api/v1/stacks/apply",
            new StringContent($$"""{"id":"{{importedStackId}}","recreate":false}""", Encoding.UTF8, "application/json"),
            cancellationToken);
        var applyBody = await applyResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(applyResponse.IsSuccessStatusCode, applyBody);
        Assert.NotNull(dispatchedCommand);
        Assert.True(dispatchedCommand.ConvertComposeProjectToSwarm);
        Assert.Equal(StackOrchestrationMode.DockerSwarm, dispatchedCommand.OrchestrationMode);
    }

    [Fact]
    public async Task ImportEndpoint_ShouldAtomicallyLinkTheWholeNamespaceWithoutMutatingDocker()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        using var draftResponse = await Client.GetAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/sample",
            cancellationToken);
        Assert.Equal(HttpStatusCode.OK, draftResponse.StatusCode);

        const string compose = "services:\n  web:\n    image: nginx:latest\n";
        var validateJson = $$"""
            {
              "name": "sample",
              "stackSource": "WebEditor",
              "spec": {
                "$type": "WebEditor",
                "composeFile": {{JsonSerializer.Serialize(compose)}},
                "updateBehavior": "Disabled",
                "projectName": "sample",
                "destroyBeforeDeploy": false
              }
            }
            """;
        using var validateResponse = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/sample/import-draft",
            new StringContent(validateJson, Encoding.UTF8, "application/json"),
            cancellationToken);
        Assert.Equal(HttpStatusCode.OK, validateResponse.StatusCode);
        using var validation = JsonDocument.Parse(await validateResponse.Content.ReadAsStringAsync(cancellationToken));
        var fingerprint = validation.RootElement.GetProperty("previewFingerprint").GetString();
        Assert.False(string.IsNullOrWhiteSpace(fingerprint));

        var importJson = $$"""
            {
              "name": "sample",
              "description": "Imported for integration test",
              "stackSource": "WebEditor",
              "spec": {
                "$type": "WebEditor",
                "composeFile": {{JsonSerializer.Serialize(compose)}},
                "updateBehavior": "Disabled",
                "projectName": "sample",
                "destroyBeforeDeploy": false
              },
              "previewFingerprint": "{{fingerprint}}"
            }
            """;
        using var importResponse = await Client.PostAsync(
            $"/api/v1/platforms/{platformId}/unmanaged-compose-projects/sample/import",
            new StringContent(importJson, Encoding.UTF8, "application/json"),
            cancellationToken);
        var body = await importResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(importResponse.IsSuccessStatusCode, body);

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = Assert.Single(await uow.Stacks.GetInfoAsync(cancellationToken));
            importedStackId = stack.Id;
            importedReleaseId = stack.CurrentStackReleaseId;
            Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
            var reservation = await uow.Stacks.GetSwarmNamespaceReservationAsync(stack.Id, cancellationToken);
            Assert.Equal("sample", reservation?.Namespace);
            var projection = Assert.Single(await uow.Swarm.GetServicesAsync(platformId, cancellationToken));
            Assert.Equal(stack.Id, projection.StackId);
            Assert.Equal(SwarmServiceOwnership.CitadelStack, projection.Ownership);
            var taskContainer = await uow.Containers.GetByIdAsync(TaskContainerId, cancellationToken);
            Assert.NotNull(taskContainer);
            Assert.True(taskContainer.IsSwarmTask);
            Assert.Equal(stack.Id, taskContainer.StackId);
        }

        connector.Verify(value => value.ListServicesAsync(It.IsAny<ListSwarmServicesCommand>(), It.IsAny<CancellationToken>()), Times.AtLeast(3));
        connector.Verify(value => value.DeleteInventoryServiceAsync(It.IsAny<DeleteSwarmInventoryServiceCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        connector.Verify(value => value.CreateServiceAsync(It.IsAny<CreateManagedSwarmServiceCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        connector.Verify(value => value.UpdateServiceAsync(It.IsAny<UpdateManagedSwarmServiceCommand>(), It.IsAny<CancellationToken>()), Times.Never);
        stackConnector.Verify(
            value => value.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()),
            Times.Never);

        using var applyResponse = await Client.PostAsync(
            "/api/v1/stacks/apply",
            new StringContent($$"""{"id":"{{importedStackId}}","recreate":false}""", Encoding.UTF8, "application/json"),
            cancellationToken);
        var applyBody = await applyResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(applyResponse.IsSuccessStatusCode, applyBody);
        Assert.True(
            applyBody.Contains("Swarm Stack converged successfully", StringComparison.Ordinal),
            applyBody);
        stackConnector.Verify(
            value => value.StackApplyAsync(
                It.Is<StackApplyCommand>(command =>
                    command.OrchestrationMode == StackOrchestrationMode.DockerSwarm
                    && command.ProjectName == "sample"),
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static SwarmServiceResult CreateService()
        => new(
            "service-1",
            7,
            "sample_web",
            "Replicated",
            "nginx:latest",
            2,
            2,
            "Completed",
            null,
            ["80/tcp"],
            [],
            [],
            [],
            new Dictionary<string, string> { ["com.docker.stack.namespace"] = "sample" },
            DateTimeOffset.UtcNow,
            DateTimeOffset.UtcNow,
            RuntimeHash: "runtime-hash");

    private SwarmServiceResult CreateOwnedService()
        => CreateService() with
        {
            Name = deployedServiceName,
            Labels = new Dictionary<string, string>
            {
                ["com.docker.stack.namespace"] = deployedStackNamespace,
                [CitadelLabels.Managed] = "true",
                [CitadelLabels.StackId] = importedStackId.ToString("D"),
                [CitadelLabels.ReleaseId] = importedReleaseId.ToString("D")
            }
        };

    private async IAsyncEnumerable<StackApplyResult> SuccessfulApplyStream(
        StackApplyCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        cancellationToken.ThrowIfCancellationRequested();
        var releaseMatch = Regex.Match(
            command.ComposeFileContent ?? string.Empty,
            $"{Regex.Escape(CitadelLabels.ReleaseId)}\\s*:\\s*['\\\"]?(?<releaseId>[0-9a-fA-F-]{{36}})",
            RegexOptions.CultureInvariant);
        Assert.True(releaseMatch.Success, "The Swarm Compose payload should contain the current Citadel release label.");
        importedReleaseId = Guid.Parse(releaseMatch.Groups["releaseId"].Value);
        deployedServiceName = $"{command.ProjectName}_web";
        deployedStackNamespace = command.ProjectName ?? command.StackName;
        stackDeployDispatched = true;
        yield return StackApplyResult.Finished(0);
    }

    private static ContainerInspectionInfo CreateComposeInspection(string containerId)
        => new(
            Id: containerId,
            Created: DateTimeOffset.UtcNow.ToString("O"),
            Path: null,
            Args: [],
            State: null,
            Image: "sha256:nginx",
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: "/composeapp-web-1",
            RestartCount: 0,
            Driver: null,
            Platform: null,
            MountLabel: null,
            ProcessLabel: null,
            AppArmorProfile: null,
            ExecIDs: [],
            HostConfig: null,
            GraphDriver: null,
            SizeRw: null,
            SizeRootFs: null,
            Mounts: [],
            Config: new ContainerConfiguration(
                Hostname: null,
                Domainname: null,
                User: null,
                AttachStdin: null,
                AttachStdout: null,
                AttachStderr: null,
                ExposedPorts: null,
                Tty: null,
                OpenStdin: null,
                StdinOnce: null,
                Env: ["TOKEN=runtime-token-value"],
                Cmd: [],
                Image: "nginx:latest",
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: new Dictionary<string, string>
                {
                    ["com.docker.compose.project"] = "composeapp",
                    ["com.docker.compose.service"] = "web",
                    ["com.docker.compose.image"] = "nginx:latest"
                }),
            NetworkSettings: null);
}
