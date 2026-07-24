using System.Collections.Immutable;
using System.Runtime.CompilerServices;
using System.Threading.Channels;
using Process = System.Diagnostics.Process;
using ProcessStartInfo = System.Diagnostics.ProcessStartInfo;
using Application.Services;
using Application.Services.Alerts;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources;
using Domain.Contracts.Resources.ResourceBindings;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Stacks;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class ApplyStackServiceTests
{
    [Fact]
    public async Task ApplyAsync_Should_Materialize_Real_GitStack_Source_From_Local_Repository()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"citadel-test-{Guid.NewGuid():N}");
        Directory.CreateDirectory(tempPath);
        var repoRoot = Path.Combine(tempPath, "repo");
        Directory.CreateDirectory(Path.Combine(repoRoot, "stacks", "app"));
        await File.WriteAllTextAsync(
            Path.Combine(repoRoot, "stacks", "app", "compose.yml"),
            "services:\n  app:\n    image: nginx\n    environment:\n      APP_MODE: ${APP_MODE}\n      API_KEY: ${API_KEY}\n",
            TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(
            Path.Combine(repoRoot, "stacks", "app", ".env"),
            "APP_ENV=repo\n",
            TestContext.Current.CancellationToken);
        await RunGitAsync(repoRoot, "init");
        await RunGitAsync(repoRoot, "checkout", "-b", "main");
        await RunGitAsync(repoRoot, "config", "user.email", "citadel@example.test");
        await RunGitAsync(repoRoot, "config", "user.name", "Citadel Test");
        await RunGitAsync(repoRoot, "add", ".");
        await RunGitAsync(repoRoot, "commit", "-m", "initial");

        var expectedCommit = (await RunGitAsync(repoRoot, "rev-parse", "HEAD")).Trim();
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var repository = new GitRepository(
            name: "local-homelab",
            description: null,
            url: new Uri(repoRoot).AbsoluteUri,
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);
        var repositoryCachePath = repository.GetCachePath();
        Directory.CreateDirectory(Path.GetDirectoryName(repositoryCachePath)!);
        if (Directory.Exists(repositoryCachePath))
        {
            DeleteDirectoryIfExists(repositoryCachePath);
        }

        var stack = Stack.Create(
            name: "git-stack-real",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: new GitStack(
                GitRepoId: repository.Id,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: ["stacks/app/compose.yml"],
                WorkingDirectory: "stacks/app",
                ComposeEnvFilesFromRepo: ["stacks/app/.env"],
                WatchPaths: ["stacks/app/**"]));

        try
        {
            StackApplyCommand? capturedCommand = null;
            var appliedContainer = new DockerContainer(
                Name: "/git-stack-real-app-1",
                Image: "nginx:latest",
                Id: "container-real",
                ImageId: "sha256:nginx",
                State: ContainerStateStatus.Running,
                Stack: "git-stack-real");

            var unitOfWork = CreateApplyUnitOfWork(stack, repository, platformId, actorId);
            var services = new ServiceCollection()
                .AddSingleton(unitOfWork.Object)
                .BuildServiceProvider();

            var containerConnector = new Mock<IContainerConnector>();
            containerConnector
                .SetupSequence(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
                .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(new Dictionary<string, DockerContainer>()))
                .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                    new Dictionary<string, DockerContainer> { [appliedContainer.Id] = appliedContainer }));

            var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
            containerConnectorFactory
                .Setup(x => x.GetConnector(PlatformConnectorType.Local))
                .Returns(containerConnector.Object);

            var stackConnector = new Mock<IStackConnector>();
            stackConnector
                .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
                .Callback<StackApplyCommand, CancellationToken>((command, _) => capturedCommand = command)
                .Returns(SuccessfulStackApplyStream());

            var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
            stackConnectorFactory
                .Setup(x => x.GetConnector(PlatformConnectorType.Local))
                .Returns(stackConnector.Object);

            var gitCli = new GitCliRepository(new ProcessCommandExecutor());
            var repoCacheManager = new RepoCacheManager(gitCli, NullLogger<RepoCacheManager>.Instance);
            var gitStackMaterializer = new GitStackMaterializer(
                repoCacheManager,
                gitCli,
                new TestStackStoragePathProvider(Path.Combine(tempPath, "stacks-storage")));

            var service = new ApplyStackService(
                new InlineDbWorkQueue(unitOfWork.Object),
                Mock.Of<IStackStreamManager>(),
                services.GetRequiredService<IServiceScopeFactory>(),
                Mock.Of<IActivityStreamManager>(),
                new TestNotificationQueue(),
                new TestPlatformContainerCache(new PlatformCacheEntry(
                    platformId,
                    "http://docker.local",
                    PlatformConnectorType.Local,
                    ImmutableDictionary<string, Guid>.Empty)),
                stackConnectorFactory.Object,
                containerConnectorFactory.Object,
                gitStackMaterializer,
                new StaticResourceBindingResolver(new ResolvedResourceBindings(
                    ["APP_MODE=prod", "API_KEY=super-secret", "UNUSED_FLAG=true", "UNUSED_SECRET=unused-secret"],
                    [
                        new ResolvedResourceBinding("APP_MODE", ResourceBindingKind.Variable, "prod"),
                        new ResolvedResourceBinding(
                            "API_KEY",
                            ResourceBindingKind.Secret,
                            "super-secret",
                            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable),
                        new ResolvedResourceBinding("UNUSED_FLAG", ResourceBindingKind.Variable, "true"),
                        new ResolvedResourceBinding(
                            "UNUSED_SECRET",
                            ResourceBindingKind.Secret,
                            "unused-secret",
                            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable)
                    ],
                    ["super-secret", "unused-secret"],
                    VariableCount: 2,
                    SecretCount: 2)
                {
                    SnapshotEntries =
                    [
                        new ResourceBindingSnapshot(
                            Name: "APP_MODE",
                            Kind: ResourceBindingKind.Variable,
                            Scope: ResourceBindingScope.Stack,
                            Value: "prod",
                            SecretId: null,
                            SecretDeliveryMode: null,
                            TargetPath: null),
                        new ResourceBindingSnapshot(
                            Name: "API_KEY",
                            Kind: ResourceBindingKind.Secret,
                            Scope: ResourceBindingScope.Stack,
                            Value: "********",
                            SecretId: Guid.CreateVersion7(),
                            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                            TargetPath: null),
                        new ResourceBindingSnapshot(
                            Name: "UNUSED_FLAG",
                            Kind: ResourceBindingKind.Variable,
                            Scope: ResourceBindingScope.Stack,
                            Value: "true",
                            SecretId: null,
                            SecretDeliveryMode: null,
                            TargetPath: null),
                        new ResourceBindingSnapshot(
                            Name: "UNUSED_SECRET",
                            Kind: ResourceBindingKind.Secret,
                            Scope: ResourceBindingScope.Stack,
                            Value: "********",
                            SecretId: Guid.CreateVersion7(),
                            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                            TargetPath: null)
                    ]
                }),
                new PassThroughSecretRedactor(),
                Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

            var items = new List<StackStreamItem>();
            await foreach (var item in service.ApplyAsync(
                stack.Id,
                actorId,
                serviceNames: null,
                pullImages: true,
                recreate: false,
                waitForCompletion: true,
                operation: StackApplyOperation.Apply,
                previousStackSnapshot: null,
                TestContext.Current.CancellationToken))
            {
                items.Add(item);
            }

            Assert.True(
                capturedCommand is not null,
                string.Join(
                    Environment.NewLine,
                    items.Select(item => item.Message ?? item.ProgressMessage ?? item.Type.ToString())));
            Assert.Null(capturedCommand!.ComposeFileContent);
            Assert.Equal("http://docker.local", capturedCommand.PlatformAddress);
            Assert.EndsWith(Path.Combine("source", "stacks", "app"), capturedCommand.SourceWorkingDirectory);
            var composePath = Assert.Single(capturedCommand.SourceComposeFilePaths!);
            var envPath = Assert.Single(capturedCommand.SourceEnvFilePaths!);
            Assert.EndsWith(Path.Combine("source", "stacks", "app", "compose.yml"), composePath);
            Assert.EndsWith(Path.Combine("source", "stacks", "app", ".env"), envPath);
            Assert.True(File.Exists(composePath));
            Assert.True(File.Exists(envPath));
            Assert.True(File.Exists(capturedCommand.LabelsOverrideFilePath));
            Assert.Equal(["APP_MODE=prod", "API_KEY=super-secret"], capturedCommand.EnvironmentVariables);
            Assert.Equal(expectedCommit, stack.CurrentStackRelease?.Source?.ResolvedCommitSha);
            Assert.Equal(["stacks/app/compose.yml"], stack.CurrentStackRelease?.Source?.ComposePaths);
            Assert.Equal(["stacks/app/.env"], stack.CurrentStackRelease?.Source?.EnvFilePaths);
            Assert.Equal("stacks/app", stack.CurrentStackRelease?.Source?.WorkingDirectory);
            Assert.Collection(
                stack.CurrentStackRelease?.ResourceBindings ?? [],
                entry =>
                {
                    Assert.Equal("APP_MODE", entry.Name);
                    Assert.Equal("prod", entry.Value);
                },
                entry =>
                {
                    Assert.Equal("API_KEY", entry.Name);
                    Assert.Equal("********", entry.Value);
                });
            Assert.Contains(
                items,
                item => item.ProgressMessage == "Resolved 1 variable APP_MODE=prod and 1 secret API_KEY for compose interpolation. Included 1 repo env file.");
            Assert.Contains(items, item => item.ExitCode == 0);
        }
        finally
        {
            DeleteDirectoryIfExists(repositoryCachePath);
            DeleteDirectoryIfExists(tempPath);
        }
    }

    [Fact]
    public async Task ApplyAsync_Should_Apply_GitStack_With_Materialized_Source_And_Activate_Snapshot()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: new GitStack(
                GitRepoId: repositoryId,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: ["stacks/app/compose.yml"],
                WorkingDirectory: "stacks/app",
                ComposeEnvFilesFromRepo: ["stacks/app/.env"]));
        var repository = new GitRepository(
            name: "homelab",
            description: null,
            url: "https://github.com/org/homelab.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);

        var snapshotRoot = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "source");
        var sourceWorkingDirectory = Path.Combine(snapshotRoot, "stacks", "app");
        var sourceComposeFile = Path.Combine(sourceWorkingDirectory, "compose.yml");
        var sourceEnvFile = Path.Combine(sourceWorkingDirectory, ".env");
        var generatedDirectory = Path.Combine(Path.GetDirectoryName(snapshotRoot)!, "citadel");
        var labelsOverrideFile = Path.Combine(generatedDirectory, "citadel.labels.yml");
        Directory.CreateDirectory(sourceWorkingDirectory);
        await File.WriteAllTextAsync(
            sourceComposeFile,
            "services:\n  app:\n    image: nginx\n",
            TestContext.Current.CancellationToken);
        await File.WriteAllTextAsync(
            sourceEnvFile,
            "APP_ENV=prod\n",
            TestContext.Current.CancellationToken);

        StackApplyCommand? capturedCommand = null;
        var appliedContainer = new DockerContainer(
            Name: "/git-stack-app-1",
            Image: "nginx:latest",
            Id: "container-1",
            ImageId: "sha256:nginx",
            State: ContainerStateStatus.Running,
            Stack: "git-stack");

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.GetContainerIdsAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        stacks
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => [stack.CurrentStackRelease!]);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos
            .Setup(x => x.GetWithAccountAsync(repositoryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);

        var containers = new Mock<IContainerRepository>();
        containers
            .SetupSequence(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([])
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .SetupSequence(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer> { [appliedContainer.Id] = appliedContainer }));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        var stackConnector = new Mock<IStackConnector>();
        stackConnector
            .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Callback<StackApplyCommand, CancellationToken>((command, _) => capturedCommand = command)
            .Returns(SuccessfulStackApplyStream());

        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(stackConnector.Object);

        var gitStackMaterializer = new Mock<IGitStackMaterializer>();
        gitStackMaterializer
            .Setup(x => x.MaterializeAsync(stack, It.IsAny<GitStack>(), repository, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new GitStackMaterializationResult(
                ResolvedCommitSha: "abc123",
                SourceBranch: "main",
                SnapshotRoot: snapshotRoot,
                SourceWorkingDirectory: sourceWorkingDirectory,
                GeneratedFilesDirectory: generatedDirectory,
                LabelsOverrideFilePath: labelsOverrideFile,
                EnvFilePath: "runtime.env",
                EnvironmentVariables: ["APP_ENV=prod"],
                SourceComposeFilePaths: [sourceComposeFile],
                SourceEnvFilePaths: [sourceEnvFile],
                ComposePaths: ["stacks/app/compose.yml"],
                EnvFilePaths: ["stacks/app/.env"],
                WatchPaths: ["stacks/app/**"])));
        gitStackMaterializer
            .Setup(x => x.ActivateCurrentAsync(stack.Id, snapshotRoot, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        gitStackMaterializer
            .Setup(x => x.PruneSnapshotsAsync(
                stack.Id,
                It.Is<IReadOnlyCollection<Guid>>(ids => ids.Contains(stack.CurrentStackReleaseId)),
                It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            gitStackMaterializer.Object,
            new EmptyResourceBindingResolver(),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        var items = new List<StackStreamItem>();
        await foreach (var item in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.NotNull(capturedCommand);
        Assert.Null(capturedCommand!.ComposeFileContent);
        Assert.Equal("http://docker.local", capturedCommand.PlatformAddress);
        Assert.Equal(sourceWorkingDirectory, capturedCommand.SourceWorkingDirectory);
        Assert.Equal([sourceComposeFile], capturedCommand.SourceComposeFilePaths);
        Assert.Equal([sourceEnvFile], capturedCommand.SourceEnvFilePaths);
        Assert.Equal(labelsOverrideFile, capturedCommand.LabelsOverrideFilePath);
        Assert.Equal(generatedDirectory, capturedCommand.GeneratedFilesDirectory);
        Assert.Equal("runtime.env", capturedCommand.EnvironmentFilePath);
        Assert.Equal(["APP_ENV=prod"], capturedCommand.EnvironmentVariables);
        Assert.True(capturedCommand.PullImages);
        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.Equal("abc123", stack.CurrentStackRelease?.Source?.ResolvedCommitSha);
        Assert.Contains(items, item => item.ExitCode == 0);
        gitStackMaterializer.Verify(x => x.ActivateCurrentAsync(stack.Id, snapshotRoot, It.IsAny<CancellationToken>()), Times.Once);
        gitStackMaterializer.Verify(x => x.DiscardSnapshotAsync(It.IsAny<Guid>(), It.IsAny<Guid>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task ApplyAsync_WhenGitStackMaterializationFails_Should_Not_Persist_Configuration_Snapshot()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: new GitStack(
                GitRepoId: repositoryId,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: ["missing-compose.yml"]));
        var repository = new GitRepository(
            name: "homelab",
            description: null,
            url: "https://github.com/org/homelab.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);

        ActivityEvent? activity = null;

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        stacks.Setup(x => x.GetContainerIdsAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        stacks.Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(() => [stack.CurrentStackRelease!]);
        stacks.Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>())).ReturnsAsync(1);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetWithAccountAsync(repositoryId, It.IsAny<CancellationToken>())).ReturnsAsync(repository);

        var containers = new Mock<IContainerRepository>();
        containers.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var images = new Mock<IImageRepository>();
        images.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);
        var actors = new Mock<IActorRepository>();
        actors.Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>())).ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);

        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(new Dictionary<string, DockerContainer>()));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Local)).Returns(containerConnector.Object);

        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Local)).Returns(Mock.Of<IStackConnector>());

        var gitStackMaterializer = new Mock<IGitStackMaterializer>();
        gitStackMaterializer
            .Setup(x => x.MaterializeAsync(stack, It.IsAny<GitStack>(), repository, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure<GitStackMaterializationResult>("compose path missing"));

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            gitStackMaterializer.Object,
            new StaticResourceBindingResolver(new ResolvedResourceBindings(
                ["API_KEY=super-secret"],
                [new ResolvedResourceBinding("API_KEY", ResourceBindingKind.Secret, "super-secret")],
                ["super-secret"],
                VariableCount: 0,
                SecretCount: 1)
            {
                SnapshotEntries =
                [
                    new ResourceBindingSnapshot(
                        Name: "API_KEY",
                        Kind: ResourceBindingKind.Secret,
                        Scope: ResourceBindingScope.Stack,
                        Value: "********",
                        SecretId: Guid.CreateVersion7(),
                        SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                        TargetPath: null)
                ]
            }),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        var items = new List<StackStreamItem>();
        await foreach (var item in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.Message?.Contains("compose path missing", StringComparison.Ordinal) == true);
        var applied = Assert.IsType<StackApplied>(activity?.Info);
        Assert.Null(applied.Result.ResourceBindings);
    }

    [Fact]
    public async Task ApplyAsync_WhenGitStackDeployFails_Should_Discard_New_Snapshot_And_Not_Activate()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: new GitStack(
                GitRepoId: repositoryId,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: ["stacks/app/compose.yml"],
                WorkingDirectory: "stacks/app"));
        var previousSource = new StackReleaseSource(
            SourceType: StackSource.Git,
            GitRepositoryId: repositoryId,
            GitRepositoryName: "homelab",
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: "old-commit",
            ComposePaths: ["stacks/app/compose.yml"],
            EnvFilePaths: [],
            WorkingDirectory: "stacks/app");
        stack.CurrentStackRelease!.UpdateSource(previousSource);
        var releaseId = stack.CurrentStackReleaseId;

        var repository = new GitRepository(
            name: "homelab",
            description: null,
            url: "https://github.com/org/homelab.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);

        var snapshotRoot = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"), "source");
        var sourceWorkingDirectory = Path.Combine(snapshotRoot, "stacks", "app");
        var sourceComposeFile = Path.Combine(sourceWorkingDirectory, "compose.yml");
        var generatedDirectory = Path.Combine(Path.GetDirectoryName(snapshotRoot)!, "citadel");
        var labelsOverrideFile = Path.Combine(generatedDirectory, "citadel.labels.yml");
        Directory.CreateDirectory(sourceWorkingDirectory);
        await File.WriteAllTextAsync(
            sourceComposeFile,
            "services:\n  app:\n    image: nginx\n",
            TestContext.Current.CancellationToken);

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(stack);
        stacks.Setup(x => x.GetContainerIdsAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        stacks.Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>())).ReturnsAsync(() => [stack.CurrentStackRelease!]);
        stacks.Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>())).ReturnsAsync(1);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos.Setup(x => x.GetWithAccountAsync(repositoryId, It.IsAny<CancellationToken>())).ReturnsAsync(repository);

        var containers = new Mock<IContainerRepository>();
        containers.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var images = new Mock<IImageRepository>();
        images.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>())).ReturnsAsync([]);
        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents.Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>())).ReturnsAsync(1);
        var actors = new Mock<IActorRepository>();
        actors.Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>())).ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);

        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Local)).Returns(Mock.Of<IContainerConnector>());

        var stackConnector = new Mock<IStackConnector>();
        stackConnector
            .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Returns(FailingStackApplyStream());
        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory.Setup(x => x.GetConnector(PlatformConnectorType.Local)).Returns(stackConnector.Object);

        var gitStackMaterializer = new Mock<IGitStackMaterializer>();
        gitStackMaterializer
            .Setup(x => x.MaterializeAsync(stack, It.IsAny<GitStack>(), repository, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(new GitStackMaterializationResult(
                ResolvedCommitSha: "new-commit",
                SourceBranch: "main",
                SnapshotRoot: snapshotRoot,
                SourceWorkingDirectory: sourceWorkingDirectory,
                GeneratedFilesDirectory: generatedDirectory,
                LabelsOverrideFilePath: labelsOverrideFile,
                EnvFilePath: null,
                EnvironmentVariables: [],
                SourceComposeFilePaths: [sourceComposeFile],
                SourceEnvFilePaths: [],
                ComposePaths: ["stacks/app/compose.yml"],
                EnvFilePaths: [],
                WatchPaths: ["stacks/app/**"])));
        gitStackMaterializer
            .Setup(x => x.DiscardSnapshotAsync(stack.Id, releaseId, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            gitStackMaterializer.Object,
            new EmptyResourceBindingResolver(),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        var items = new List<StackStreamItem>();
        await foreach (var item in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.ExitCode == 1);
        gitStackMaterializer.Verify(x => x.DiscardSnapshotAsync(stack.Id, releaseId, It.IsAny<CancellationToken>()), Times.Once);
        gitStackMaterializer.Verify(x => x.ActivateCurrentAsync(It.IsAny<Guid>(), It.IsAny<string>(), It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task ApplyAsync_Should_Redeploy_Healthy_ManualStack_Without_Creating_New_Release_When_Definition_Is_Unchanged()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "manual-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.ReleaseProcessing(StackReleaseStatus.Healthy);
        var releaseId = stack.CurrentStackReleaseId;
        var releaseVersion = stack.CurrentStackRelease!.Version;

        var appliedContainer = new DockerContainer(
            Name: "/manual-stack-app-1",
            Image: "nginx:latest",
            Id: "manual-stack-app-container",
            ImageId: "sha256:nginx",
            State: ContainerStateStatus.Running,
            Created: 123,
            Stack: "manual-stack");

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .SetupSequence(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer> { [appliedContainer.Id] = appliedContainer }));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        var stackConnector = new Mock<IStackConnector>();
        stackConnector
            .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Returns(SuccessfulStackApplyStream());

        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(stackConnector.Object);

        var repository = new GitRepository(
            name: "unused",
            description: null,
            url: "https://example.invalid/repo.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);
        var unitOfWork = CreateApplyUnitOfWork(stack, repository, platformId, actorId);
        var services = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            Mock.Of<IGitStackMaterializer>(),
            new EmptyResourceBindingResolver(),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        var items = new List<StackStreamItem>();
        await foreach (var item in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.ExitCode == 0);
        Assert.Equal(releaseId, stack.CurrentStackReleaseId);
        Assert.Equal(releaseVersion, stack.CurrentStackRelease!.Version);
        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease.Status);
    }

    [Fact]
    public async Task ApplyAsync_Should_Clear_Processing_When_Degraded_ManualStack_Is_Redeployed_Again()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "minio-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  minio:\n    image: minio/minio\n  minio-init:\n    image: minio/mc\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                DestroyBeforeDeploy: false));

        var runningContainer = new DockerContainer(
            Name: "/minio-stack-minio-1",
            Image: "minio/minio:latest",
            Id: "minio-container-id",
            ImageId: "sha256:minio",
            State: ContainerStateStatus.Running,
            Created: 123,
            Stack: "minio-stack");
        var exitedContainer = new DockerContainer(
            Name: "/minio-stack-minio-init-1",
            Image: "minio/mc:latest",
            Id: "minio-init-container-id",
            ImageId: "sha256:minio-mc",
            State: ContainerStateStatus.Exited,
            Created: 124,
            Stack: "minio-stack");

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .Setup(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>
                {
                    [runningContainer.Id] = runningContainer,
                    [exitedContainer.Id] = exitedContainer
                }));
        containerConnector
            .Setup(x => x.InspectAsync(It.IsAny<InspectContainerCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success(InspectionWithLabels(new Dictionary<string, string>())));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        var capturedCommands = new List<StackApplyCommand>();
        var stackConnector = new Mock<IStackConnector>();
        stackConnector
            .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Callback<StackApplyCommand, CancellationToken>((command, _) => capturedCommands.Add(command))
            .Returns(() => DegradedStackApplyStream());

        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(stackConnector.Object);

        var repository = new GitRepository(
            name: "unused",
            description: null,
            url: "https://example.invalid/repo.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);
        var unitOfWork = CreateApplyUnitOfWork(
            stack,
            repository,
            platformId,
            actorId,
            [runningContainer.Id, exitedContainer.Id]);
        var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            Mock.Of<IGitStackMaterializer>(),
            new EmptyResourceBindingResolver(),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        await foreach (var _ in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
        }

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease!.Status);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);

        await foreach (var _ in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: true,
            recreate: true,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
        }

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease!.Status);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        Assert.Collection(
            capturedCommands,
            command => Assert.False(command.DestroyBeforeDeploy),
            command => Assert.True(command.DestroyBeforeDeploy));
    }

    [Fact]
    public async Task ApplyAsync_Should_Pass_Mounted_File_Secrets_To_Stack_Command_Without_Env_Injection()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "postgres-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile:
                """
                services:
                  db:
                    image: postgres:16
                    environment:
                      APP_MODE: ${APP_MODE}
                      POSTGRES_PASSWORD_FILE: /run/secrets/postgres_password
                """,
                UpdateBehavior: StackUpdateBehavior.Disabled));

        var appliedContainer = new DockerContainer(
            Name: "/postgres-stack-db-1",
            Image: "postgres:16",
            Id: "postgres-container",
            ImageId: "sha256:postgres",
            State: ContainerStateStatus.Running,
            Created: 123,
            Stack: "postgres-stack");

        var containerConnector = new Mock<IContainerConnector>();
        containerConnector
            .SetupSequence(x => x.ListContainersAsync(It.IsAny<ContainerFilterCommand>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer>()))
            .ReturnsAsync(Result.Success<IReadOnlyDictionary<string, DockerContainer>>(
                new Dictionary<string, DockerContainer> { [appliedContainer.Id] = appliedContainer }));

        var containerConnectorFactory = new Mock<IConnectorFactory<IContainerConnector>>();
        containerConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(containerConnector.Object);

        StackApplyCommand? capturedCommand = null;
        var stackConnector = new Mock<IStackConnector>();
        stackConnector
            .Setup(x => x.StackApplyAsync(It.IsAny<StackApplyCommand>(), It.IsAny<CancellationToken>()))
            .Callback<StackApplyCommand, CancellationToken>((command, _) => capturedCommand = command)
            .Returns(SuccessfulStackApplyStream());

        var stackConnectorFactory = new Mock<IConnectorFactory<IStackConnector>>();
        stackConnectorFactory
            .Setup(x => x.GetConnector(PlatformConnectorType.Local))
            .Returns(stackConnector.Object);

        var repository = new GitRepository(
            name: "unused",
            description: null,
            url: "https://example.invalid/repo.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: actorId);
        var unitOfWork = CreateApplyUnitOfWork(stack, repository, platformId, actorId);
        var services = new ServiceCollection()
            .AddScoped(_ => unitOfWork.Object)
            .BuildServiceProvider();
        var secretId = Guid.CreateVersion7();

        var service = new ApplyStackService(
            new InlineDbWorkQueue(unitOfWork.Object),
            Mock.Of<IStackStreamManager>(),
            services.GetRequiredService<IServiceScopeFactory>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            new TestPlatformContainerCache(new PlatformCacheEntry(
                platformId,
                "http://docker.local",
                PlatformConnectorType.Local,
                ImmutableDictionary<string, Guid>.Empty)),
            stackConnectorFactory.Object,
            containerConnectorFactory.Object,
            Mock.Of<IGitStackMaterializer>(),
            new StaticResourceBindingResolver(new ResolvedResourceBindings(
                EnvironmentVariables: ["APP_MODE=prod"],
                Entries:
                [
                    new ResolvedResourceBinding("APP_MODE", ResourceBindingKind.Variable, "prod"),
                    new ResolvedResourceBinding(
                        "POSTGRES_PASSWORD",
                        ResourceBindingKind.Secret,
                        "super-secret",
                        SecretName: "postgres-password",
                        SecretDeliveryMode: SecretDeliveryMode.MountedFile,
                        TargetPath: "/run/secrets/postgres_password")
                ],
                RedactionValues: ["super-secret"],
                VariableCount: 1,
                SecretCount: 1)
            {
                SnapshotEntries =
                [
                    new ResourceBindingSnapshot(
                        Name: "APP_MODE",
                        Kind: ResourceBindingKind.Variable,
                        Scope: ResourceBindingScope.Stack,
                        Value: "prod",
                        SecretId: null,
                        SecretDeliveryMode: null,
                        TargetPath: null),
                    new ResourceBindingSnapshot(
                        Name: "POSTGRES_PASSWORD",
                        Kind: ResourceBindingKind.Secret,
                        Scope: ResourceBindingScope.Stack,
                        Value: "********",
                        SecretId: secretId,
                        SecretDeliveryMode: SecretDeliveryMode.MountedFile,
                        TargetPath: "/run/secrets/postgres_password")
                ]
            }),
            new PassThroughSecretRedactor(),
            Mock.Of<IAlertService>(),
            EmptyStackBuildImageBindingResolver.Instance);

        var items = new List<StackStreamItem>();
        await foreach (var item in service.ApplyAsync(
            stack.Id,
            actorId,
            serviceNames: null,
            pullImages: false,
            recreate: false,
            waitForCompletion: true,
            operation: StackApplyOperation.Apply,
            previousStackSnapshot: null,
            TestContext.Current.CancellationToken))
        {
            items.Add(item);
        }

        Assert.Contains(items, item => item.ExitCode == 0);
        Assert.NotNull(capturedCommand);
        Assert.Equal(["APP_MODE=prod"], capturedCommand!.EnvironmentVariables);
        var secretFile = Assert.Single(capturedCommand.SecretFiles!);
        Assert.Equal("POSTGRES_PASSWORD", secretFile.Name);
        Assert.Equal("/run/secrets/postgres_password", secretFile.TargetPath);
        Assert.Equal("super-secret", secretFile.Content);
        Assert.Equal(["db"], capturedCommand.SecretTargetServiceNames);
        Assert.Collection(
            stack.CurrentStackRelease?.ResourceBindings ?? [],
            entry => Assert.Equal("APP_MODE", entry.Name),
            entry =>
            {
                Assert.Equal("POSTGRES_PASSWORD", entry.Name);
                Assert.Equal("********", entry.Value);
                Assert.Equal(SecretDeliveryMode.MountedFile, entry.SecretDeliveryMode);
                Assert.Equal("/run/secrets/postgres_password", entry.TargetPath);
            });
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Record_Only_ServiceScoped_Build_Provenance()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var apiBuildProjectId = Guid.CreateVersion7();
        var workerBuildProjectId = Guid.CreateVersion7();
        var apiRunId = Guid.CreateVersion7();
        var workerResolvedRunId = Guid.CreateVersion7();
        var workerAppliedRunId = Guid.CreateVersion7();
        var workerAppliedAt = DateTimeOffset.UtcNow.AddHours(-1);
        var stack = Stack.Create(
            name: "application",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  api:\n    image: api\n  worker:\n    image: worker\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                BuildImageBindings:
                [
                    new StackBuildImageBinding(
                        ServiceName: "api",
                        BuildProjectId: apiBuildProjectId,
                        ResolvedImageReference: "registry.example.test/api:42",
                        ResolvedDigest: "sha256:api",
                        ResolvedBuildRunId: apiRunId),
                    new StackBuildImageBinding(
                        ServiceName: "worker",
                        BuildProjectId: workerBuildProjectId,
                        ResolvedImageReference: "registry.example.test/worker:43",
                        ResolvedDigest: "sha256:worker-new",
                        ResolvedBuildRunId: workerResolvedRunId,
                        AppliedImageReference: "registry.example.test/worker:41",
                        AppliedDigest: "sha256:worker-old",
                        AppliedBuildRunId: workerAppliedRunId,
                        AppliedAt: workerAppliedAt)
                ]));
        stack.MarkProcessing(actorId);

        var stacks = new Mock<IStackRepository>();
        stacks.Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks.Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        stacks.Setup(x => x.ReplaceReleaseVolumeBindingsAsync(
                stack.CurrentStackRelease!.Id,
                It.IsAny<IReadOnlyCollection<StackReleaseVolumeBinding>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var containers = new Mock<IContainerRepository>();
        containers.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers.Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var images = new Mock<IImageRepository>();
        images.Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents.Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors.Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var appliedAt = DateTimeOffset.UtcNow;
        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            [],
            StackReleaseStatus.Healthy,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Apply,
            appliedBuildImages:
            [
                new AppliedStackBuildImage(
                    "api",
                    apiBuildProjectId,
                    "registry.example.test/api:42",
                    "sha256:api-runtime",
                    apiRunId)
            ]);

        await workItem.ExecuteAsync(unitOfWork.Object, TestContext.Current.CancellationToken);

        var bindings = stack.CurrentStackRelease!.Spec.BuildImageBindings!;
        var api = Assert.Single(bindings, binding => binding.ServiceName == "api");
        Assert.Equal(apiRunId, api.AppliedBuildRunId);
        Assert.Equal("registry.example.test/api:42", api.AppliedImageReference);
        Assert.Equal("sha256:api-runtime", api.AppliedDigest);
        Assert.True(api.AppliedAt >= appliedAt);

        var worker = Assert.Single(bindings, binding => binding.ServiceName == "worker");
        Assert.Equal(workerResolvedRunId, worker.ResolvedBuildRunId);
        Assert.Equal(workerAppliedRunId, worker.AppliedBuildRunId);
        Assert.Equal("sha256:worker-old", worker.AppliedDigest);
        Assert.Equal(workerAppliedAt, worker.AppliedAt);
    }

    [Fact]
    public async Task StackSucceededWorkItem_creates_and_associates_containers_when_sync_has_not_seen_them_yet()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n  beszel-agent:\n    image: henrygd/beszel-agent\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.MarkProcessing(actorId);

        var dockerContainers = new[]
        {
            new DockerContainer(
                Name: "/beszel-beszel-1",
                Image: "henrygd/beszel:latest",
                Id: "beszel-container-id",
                ImageId: "sha256:beszel",
                State: ContainerStateStatus.Running,
                Created: 123,
                Stack: "beszel"),
            new DockerContainer(
                Name: "/beszel-beszel-agent-1",
                Image: "henrygd/beszel-agent:latest",
                Id: "beszel-agent-container-id",
                ImageId: "sha256:beszel-agent",
                State: ContainerStateStatus.Running,
                Created: 124,
                Stack: "beszel")
        };

        List<Container> upsertedContainers = [];
        ActivityEvent? activity = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .Callback<IEnumerable<Container>, CancellationToken>((items, _) => upsertedContainers = items.ToList())
            .ReturnsAsync(2);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var resourceBindings = new[]
        {
            new ResourceBindingSnapshot(
                Name: "stripe_api_key",
                Kind: ResourceBindingKind.Secret,
                Scope: ResourceBindingScope.Stack,
                Value: "********",
                SecretId: Guid.CreateVersion7(),
                SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable,
                TargetPath: null)
        };
        var notificationQueue = new TestNotificationQueue();
        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            dockerContainers,
            null,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            notificationQueue,
            StackApplyOperation.Apply,
            resourceBindings: resourceBindings);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.Equal("stripe_api_key", Assert.Single(stack.CurrentStackRelease?.ResourceBindings ?? []).Name);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
        Assert.Equal(2, upsertedContainers.Count);
        Assert.All(upsertedContainers, container => Assert.Equal(stack.Id, container.StackId));
        Assert.Contains(upsertedContainers, container => container.DockerContainerId == "beszel-container-id");
        Assert.Contains(upsertedContainers, container => container.DockerContainerId == "beszel-agent-container-id");

        var applied = Assert.IsType<StackApplied>(activity?.Info);
        Assert.Equal(["beszel-container-id", "beszel-agent-container-id"], applied.Result.ContainerIds);
        Assert.Equal("stripe_api_key", Assert.Single(applied.Stack?.StackRelease?.ResourceBindings ?? []).Name);
        Assert.Equal("stripe_api_key", Assert.Single(applied.Result.ResourceBindings ?? []).Name);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Once);
        Assert.Equal(2, notificationQueue.Items.Count);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Mark_Stack_Degraded_When_Applied_Containers_Have_Mixed_States()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "minio-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  minio:\n    image: minio/minio\n  minio-init:\n    image: minio/mc\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.MarkProcessing(actorId);

        var dockerContainers = new[]
        {
            new DockerContainer(
                Name: "/minio-stack-minio-1",
                Image: "minio/minio:latest",
                Id: "minio-container-id",
                ImageId: "sha256:minio",
                State: ContainerStateStatus.Running,
                Created: 123,
                Stack: "minio-stack"),
            new DockerContainer(
                Name: "/minio-stack-minio-init-1",
                Image: "minio/mc:latest",
                Id: "minio-init-container-id",
                ImageId: "sha256:minio-mc",
                State: ContainerStateStatus.Exited,
                Created: 124,
                Stack: "minio-stack")
        };

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(2);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            dockerContainers,
            null,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Apply);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Use_Compose_Status_When_Reported()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "minio-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  minio:\n    image: minio/minio\n  minio-init:\n    image: minio/mc\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.MarkProcessing(actorId);

        var dockerContainers = new[]
        {
            new DockerContainer(
                Name: "/minio-stack-minio-1",
                Image: "minio/minio:latest",
                Id: "minio-container-id",
                ImageId: "sha256:minio",
                State: ContainerStateStatus.Running,
                Created: 123,
                Stack: "minio-stack")
        };

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            dockerContainers,
            StackReleaseStatus.Degraded,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Apply);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Degraded, stack.CurrentStackRelease?.Status);
        Assert.Equal(ResourceControlState.Idle, stack.ControlState);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Deduplicate_Volume_Bindings_Before_Persisting()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "volume-stack",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx\n    volumes:\n      - data:/data\nvolumes:\n  data:\n",
                UpdateBehavior: StackUpdateBehavior.Disabled));
        stack.MarkProcessing(actorId);

        IReadOnlyCollection<StackReleaseVolumeBinding>? persistedBindings = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        stacks
            .Setup(x => x.ReplaceReleaseVolumeBindingsAsync(
                stack.CurrentStackReleaseId,
                It.IsAny<IReadOnlyCollection<StackReleaseVolumeBinding>>(),
                It.IsAny<CancellationToken>()))
            .Callback<Guid, IReadOnlyCollection<StackReleaseVolumeBinding>, CancellationToken>((_, bindings, _) =>
                persistedBindings = bindings)
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var duplicateBindings = new[]
        {
            new StackReleaseVolumeBinding(stack.CurrentStackReleaseId, platformId, "data", "data"),
            new StackReleaseVolumeBinding(stack.CurrentStackReleaseId, platformId, "data ", "data")
        };
        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            [],
            null,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Apply,
            volumeBindings: duplicateBindings);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        var binding = Assert.Single(persistedBindings ?? []);
        Assert.Equal("data", binding.VolumeName);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Record_Rollback_Activity_For_Rollback_Operation()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "beszel",
            createdByActorId: actorId,
            StackSource: StackSource.WebEditor,
            platformId: platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.Notify));
        var previousStackSnapshot = stack.ToSnapshot();
        stack.UpdateCurrentStackReleaseDefinition(
            platformId,
            new ManualStack(
                ComposeFile: "services:\n  beszel:\n    image: henrygd/beszel\n",
                UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy));
        stack.MarkProcessing(actorId);

        ActivityEvent? activity = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            [],
            null,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Rollback,
            previousStackSnapshot);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(ActivityEventType.StackRollback, activity?.EventType);
        var rollback = Assert.IsType<StackRollback>(activity?.Info);
        Assert.Equal(StackUpdateBehavior.Notify, Assert.IsType<ManualStack>(rollback.OldStack!.StackRelease!.Spec).UpdateBehavior);
        Assert.Equal(StackUpdateBehavior.ServiceAutoDeploy, Assert.IsType<ManualStack>(rollback.NewStack!.StackRelease!.Spec).UpdateBehavior);
    }

    [Fact]
    public async Task StackSucceededWorkItem_Should_Persist_Git_Source_And_Clear_Git_Update_State()
    {
        var platformId = Guid.CreateVersion7();
        var actorId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var stack = Stack.Create(
            name: "git-stack",
            createdByActorId: actorId,
            StackSource: StackSource.Git,
            platformId: platformId,
            spec: new GitStack(
                GitRepoId: repositoryId,
                Branch: "main",
                CommitSha: null,
                UpdateBehavior: StackUpdateBehavior.Notify,
                ComposePaths: ["stacks/app/compose.yml"],
                WorkingDirectory: "stacks/app"));

        stack.SetStackUpdateState(new GitStackUpdateState(
            new RecreateStackOnNewImageState([]),
            new RecreateStackOnNewCommitState("old-commit", "new-commit", DateTime.UtcNow.AddMinutes(-5))));
        stack.MarkProcessing(actorId);

        var source = new StackReleaseSource(
            SourceType: StackSource.Git,
            GitRepositoryId: repositoryId,
            GitRepositoryName: "homelab",
            Branch: "main",
            RequestedCommitSha: null,
            ResolvedCommitSha: "new-commit",
            ComposePaths: ["stacks/app/compose.yml"],
            EnvFilePaths: ["stacks/app/.env"],
            GitRepositoryUrl: "https://github.com/org/homelab",
            WorkingDirectory: "stacks/app",
            WatchPaths: ["stacks/app/**"],
            ComposeEnvFilesFromRepo: ["stacks/app/.env"]);

        ActivityEvent? activity = null;
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(0);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .Callback<ActivityEvent, CancellationToken>((item, _) => activity = item)
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var workItem = new StackSucceededWorkItem(
            stack.Id,
            actorId,
            [],
            null,
            Mock.Of<IStackStreamManager>(),
            Mock.Of<IActivityStreamManager>(),
            new TestNotificationQueue(),
            StackApplyOperation.Apply,
            previousStackSnapshot: null,
            source);

        await workItem.ExecuteAsync(unitOfWork.Object, CancellationToken.None);

        Assert.Equal(StackReleaseStatus.Healthy, stack.CurrentStackRelease?.Status);
        Assert.Equal(source, stack.CurrentStackRelease?.Source);
        var gitState = Assert.IsType<GitStackUpdateState>(stack.StackUpdateState);
        Assert.Equal("new-commit", gitState.RecreateStackOnNewCommitState.CurrentCommitSha);
        Assert.Null(gitState.RecreateStackOnNewCommitState.RemoteCommitSha);

        var applied = Assert.IsType<StackApplied>(activity?.Info);
        Assert.Equal(source, applied.Stack?.StackRelease?.Source);
    }

    private sealed class TestNotificationQueue : INotificationQueue
    {
        public List<INotificationWorkItem> Items { get; } = [];

        public ChannelReader<INotificationWorkItem> Reader { get; } = Channel.CreateUnbounded<INotificationWorkItem>().Reader;

        public ValueTask EnqueueAsync(INotificationWorkItem item, CancellationToken ct)
        {
            Items.Add(item);
            return ValueTask.CompletedTask;
        }
    }

    private static async IAsyncEnumerable<StackApplyResult> SuccessfulStackApplyStream()
    {
        yield return StackApplyResult.StdOut("compose up");
        yield return StackApplyResult.Finished(0);
        await Task.CompletedTask;
    }

    private static async IAsyncEnumerable<StackApplyResult> DegradedStackApplyStream()
    {
        yield return StackApplyResult.StdOut("compose up");
        yield return StackApplyResult.Finished(0);
        yield return StackApplyResult.ComposeStatus(StackReleaseStatus.Degraded);
        await Task.CompletedTask;
    }

    private static async IAsyncEnumerable<StackApplyResult> FailingStackApplyStream()
    {
        yield return StackApplyResult.StdErr("compose failed");
        yield return StackApplyResult.Finished(1);
        await Task.CompletedTask;
    }

    private static ContainerInspectionInfo InspectionWithLabels(IReadOnlyDictionary<string, string> labels)
        => new(
            Id: "container-id",
            Created: string.Empty,
            Path: null,
            Args: [],
            State: null,
            Image: null,
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: null,
            RestartCount: null,
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
                Env: [],
                Cmd: [],
                Image: null,
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: labels),
            NetworkSettings: null);

    private static void DeleteDirectoryIfExists(string path)
    {
        if (!Directory.Exists(path))
            return;

        foreach (var file in Directory.EnumerateFiles(path, "*", SearchOption.AllDirectories))
        {
            File.SetAttributes(file, FileAttributes.Normal);
        }

        foreach (var directory in Directory.EnumerateDirectories(path, "*", SearchOption.AllDirectories))
        {
            File.SetAttributes(directory, FileAttributes.Normal);
        }

        File.SetAttributes(path, FileAttributes.Normal);
        Directory.Delete(path, recursive: true);
    }

    private static async Task<string> RunGitAsync(string workingDirectory, params string[] arguments)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = "git",
            WorkingDirectory = workingDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        foreach (var argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        using var process = Process.Start(startInfo)!;
        var stdout = await process.StandardOutput.ReadToEndAsync(TestContext.Current.CancellationToken);
        var stderr = await process.StandardError.ReadToEndAsync(TestContext.Current.CancellationToken);
        await process.WaitForExitAsync(TestContext.Current.CancellationToken);

        if (process.ExitCode != 0)
            throw new InvalidOperationException($"git {string.Join(' ', arguments)} failed: {stderr}");

        return stdout;
    }

    private static Mock<IUnitOfWork> CreateApplyUnitOfWork(
        Stack stack,
        GitRepository repository,
        Guid platformId,
        Guid actorId,
        IReadOnlyCollection<string>? stackContainerIds = null)
    {
        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stack);
        stacks
            .Setup(x => x.GetContainerIdsAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stackContainerIds ?? []);
        stacks
            .Setup(x => x.GetReleasesByStackIdAsync(stack.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(() => [stack.CurrentStackRelease!]);
        stacks
            .Setup(x => x.UpdateAsync(stack, It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var gitRepos = new Mock<IGitReposRepository>();
        gitRepos
            .Setup(x => x.GetWithAccountAsync(repository.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(repository);

        var containers = new Mock<IContainerRepository>();
        containers
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);
        containers
            .Setup(x => x.BulkUpsertAsync(It.IsAny<IEnumerable<Container>>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var images = new Mock<IImageRepository>();
        images
            .Setup(x => x.GetByPlatformIdAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync([]);

        var activityEvents = new Mock<IActivityEventRepository>();
        activityEvents
            .Setup(x => x.AddAsync(It.IsAny<ActivityEvent>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);

        var actors = new Mock<IActorRepository>();
        actors
            .Setup(x => x.GetById(actorId, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Actor?)null);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.Stacks).Returns(stacks.Object);
        unitOfWork.Setup(x => x.GitRepositories).Returns(gitRepos.Object);
        unitOfWork.Setup(x => x.Containers).Returns(containers.Object);
        unitOfWork.Setup(x => x.Images).Returns(images.Object);
        unitOfWork.Setup(x => x.ActivityEventRepository).Returns(activityEvents.Object);
        unitOfWork.Setup(x => x.Actors).Returns(actors.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        return unitOfWork;
    }

    private sealed class ProcessCommandExecutor : ICommandExecutor
    {
        public async Task<ProcessExecutionResult> ExecuteAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = fileName,
                WorkingDirectory = workingDirectory ?? string.Empty,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            foreach (var argument in arguments)
            {
                startInfo.ArgumentList.Add(argument);
            }

            if (environmentVariables is not null)
            {
                foreach (var (key, value) in environmentVariables)
                {
                    startInfo.Environment[key] = value;
                }
            }

            using var process = Process.Start(startInfo)!;
            var stdout = await process.StandardOutput.ReadToEndAsync(cancellationToken);
            var stderr = await process.StandardError.ReadToEndAsync(cancellationToken);
            await process.WaitForExitAsync(cancellationToken);
            return new ProcessExecutionResult(process.ExitCode, stdout, stderr);
        }

        public async IAsyncEnumerable<ProcessOutput> StreamAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            string? dockerConfigDirectory = null,
            [EnumeratorCancellation] CancellationToken cancellationToken = default)
        {
            await Task.CompletedTask;
            yield break;
        }
    }

    private sealed class InlineDbWorkQueue(IUnitOfWork unitOfWork) : IDbWorkQueue
    {
        public ChannelReader<IDbWorkItem> Reader { get; } = Channel.CreateUnbounded<IDbWorkItem>().Reader;

        public ValueTask EnqueueAsync(IDbWorkItem item, CancellationToken cancellationToken)
            => EnqueueAndWaitAsync(item, cancellationToken);

        public async ValueTask EnqueueAndWaitAsync(IDbWorkItem item, CancellationToken cancellationToken)
        {
            await item.ExecuteAsync(unitOfWork, cancellationToken);
        }
    }

    private sealed class TestPlatformContainerCache(PlatformCacheEntry platform) : IPlatformContainerCache
    {
        public void ReplacePlatformContainers(Guid platformId, PlatformCacheEntry cacheEntry)
        {
        }

        public bool TryAddContainer(Guid platformId, string containerId, Guid dbId) => false;

        public bool TryRemoveContainer(Guid platformId, string containerId) => false;

        public bool EvictPlatform(Guid platformId) => false;

        public bool TryGetContainers(Guid platformId, out IReadOnlyDictionary<string, Guid> containers)
        {
            containers = platform.Id == platformId
                ? platform.Containers
                : ImmutableDictionary<string, Guid>.Empty;
            return platform.Id == platformId;
        }

        public bool TryGetCacheEntry(Guid platformId, out PlatformCacheEntry cacheEntry, out Error error)
        {
            if (platform.Id == platformId)
            {
                cacheEntry = platform;
                error = null!;
                return true;
            }

            cacheEntry = null!;
            error = new Error("not found");
            return false;
        }

        public bool TryGetCacheEntries(out IEnumerable<PlatformCacheEntry> cacheEntries, out Error error)
        {
            cacheEntries = [platform];
            error = null!;
            return true;
        }

        public bool TryGetPlatformWithContainer(string containerId, out PlatformCacheEntry cacheEntry)
        {
            cacheEntry = null!;
            return false;
        }

        public bool TryGetPlatformsWithContainers(string[] containersId, out List<PlatformCacheEntry> cacheEntries)
        {
            cacheEntries = [];
            return false;
        }
    }

    private sealed class TestStackStoragePathProvider(string path) : IStackStoragePathProvider
    {
        public string StacksRoot { get; } = path;
    }

    private sealed class EmptyResourceBindingResolver : IResourceBindingResolver
    {
        public Task<Result<ResolvedResourceBindings>> ResolveAsync(
            ResourceBindingScope scope,
            Guid resourceId,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(new ResolvedResourceBindings([], [], [], 0, 0)));
    }

    private sealed class StaticResourceBindingResolver(ResolvedResourceBindings configuration) : IResourceBindingResolver
    {
        public Task<Result<ResolvedResourceBindings>> ResolveAsync(
            ResourceBindingScope scope,
            Guid resourceId,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(configuration));
    }

    private sealed class PassThroughSecretRedactor : ISecretRedactor
    {
        public string Redact(string? value, IEnumerable<string> secrets) => value ?? string.Empty;
    }

    private sealed class EmptyStackBuildImageBindingResolver : IStackBuildImageBindingResolver
    {
        public static EmptyStackBuildImageBindingResolver Instance { get; } = new();

        public Task<Result<ResolvedStackBuildImageBindings>> ResolveAsync(
            IReadOnlyList<StackBuildImageBinding>? bindings,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(new ResolvedStackBuildImageBindings([], [])));

        public string ApplyToComposeContent(string composeContent, IReadOnlyList<ResolvedStackBuildImageBinding> bindings)
            => composeContent;

        public string CreateComposeOverride(IReadOnlyList<ResolvedStackBuildImageBinding> bindings)
            => "services: {}\n";
    }

    private sealed class TempDirectory : IDisposable
    {
        public string Path { get; } = System.IO.Path.Combine(System.IO.Path.GetTempPath(), Guid.NewGuid().ToString("N"));

        public TempDirectory()
        {
            Directory.CreateDirectory(Path);
        }

        public void Dispose()
        {
            if (Directory.Exists(Path))
                Directory.Delete(Path, recursive: true);
        }
    }
}
