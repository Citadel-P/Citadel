using Application.Features.Backups.Commands;
using Application.Features.Backups.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Backups;
using Hosting.Common.Abstraction;
using Moq;

namespace Tests.Unit.Application.Features.Backups;

public sealed class BackupPolicyCommandsTests
{
    [Fact]
    public async Task CreateBackupPolicy_ShouldRejectCoreFilesystemRepositoryForRemoteDockerVolume()
    {
        var platformId = Guid.CreateVersion7();
        var repository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Core,
            PlatformId: null,
            Path: "remote-volume-backups"));
        var backupPolicies = CreateBackupPolicyRepository();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(platformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(platformId, "edge-01", "edge://edge-01", PlatformConnectorType.EdgeAgent));
        var unitOfWork = CreateUnitOfWork(repository, backupPolicies.Object, platforms.Object);
        var handler = CreateCreateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            CreatePolicyCommand(repository.Id, new DockerVolumeBackupSource(platformId, "app-data")),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal(
            "Core filesystem backup repositories cannot back up remote Docker volumes. Use an S3-compatible repository or a filesystem repository on the same platform.",
            error.Message);
        backupPolicies.Verify(
            x => x.AddAsync(
                It.IsAny<BackupPolicy>(),
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                It.IsAny<Guid?>()),
            Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task CreateBackupPolicy_ShouldRejectFilesystemRepositoryOnDifferentPlatform()
    {
        var sourcePlatformId = Guid.CreateVersion7();
        var repositoryPlatformId = Guid.CreateVersion7();
        var repository = CreateRepository(new FileSystemBackupRepositorySpec(
            BackupExecutionLocation.Platform,
            repositoryPlatformId,
            Path: "/backups"));
        var backupPolicies = CreateBackupPolicyRepository();
        var platforms = new Mock<IPlatformRepository>();
        platforms
            .Setup(x => x.GetInfoAsync(sourcePlatformId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PlatformConnectionInfo(sourcePlatformId, "local-01", "unix:///var/run/docker.sock", PlatformConnectorType.Local));
        var unitOfWork = CreateUnitOfWork(repository, backupPolicies.Object, platforms.Object);
        var handler = CreateCreateHandler(unitOfWork.Object);

        var result = await handler.Handle(
            CreatePolicyCommand(repository.Id, new DockerVolumeBackupSource(sourcePlatformId, "app-data")),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess(out _, out var error));
        Assert.Equal("Filesystem backup repository platform must match the backup source platform.", error.Message);
        backupPolicies.Verify(
            x => x.AddAsync(
                It.IsAny<BackupPolicy>(),
                It.IsAny<CancellationToken>(),
                It.IsAny<IReadOnlyCollection<Guid>?>(),
                It.IsAny<Guid?>()),
            Times.Never);
        unitOfWork.Verify(x => x.CommitAsync(It.IsAny<CancellationToken>()), Times.Never);
    }

    private static CreateBackupPolicy CreatePolicyCommand(Guid repositoryId, BackupSourceSpec source)
        => new(new BackupPolicyInputModel(
            Name: "daily-volume-backup",
            Description: null,
            Source: source,
            BackupRepositoryId: repositoryId,
            Enabled: true,
            Cron: null,
            TimeZone: null,
            Webhook: null,
            KeepLastSuccessful: BackupPolicy.DefaultKeepLastSuccessful,
            TimeoutSeconds: BackupPolicy.DefaultTimeoutSeconds,
            AlertOnFailure: true,
            RunAsActorId: null,
            TagIds: []));

    private static CreateBackupPolicyHandler CreateCreateHandler(IUnitOfWork unitOfWork)
        => new(
            unitOfWork,
            CreateUserContextAccessor(),
            Mock.Of<IStackBackupVolumeResolver>(),
            Mock.Of<IDeploymentBackupVolumeResolver>());

    private static Mock<IBackupPolicyRepository> CreateBackupPolicyRepository()
    {
        var backupPolicies = new Mock<IBackupPolicyRepository>();
        backupPolicies
            .Setup(x => x.ExistsByNormalizedNameAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);
        return backupPolicies;
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(
        BackupRepository repository,
        IBackupPolicyRepository backupPolicies,
        IPlatformRepository platforms)
    {
        var repositories = new Mock<IBackupRepositoryRepository>();
        repositories
            .Setup(x => x.GetAsync(repository.Id, It.IsAny<CancellationToken>(), It.IsAny<bool>()))
            .ReturnsAsync(repository);

        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.Setup(x => x.BackupRepositories).Returns(repositories.Object);
        unitOfWork.Setup(x => x.BackupPolicies).Returns(backupPolicies);
        unitOfWork.Setup(x => x.Platforms).Returns(platforms);
        unitOfWork.Setup(x => x.Stacks).Returns(Mock.Of<IStackRepository>());
        unitOfWork.Setup(x => x.Deployments).Returns(Mock.Of<IDeploymentRepository>());
        unitOfWork.Setup(x => x.CommitAsync(It.IsAny<CancellationToken>())).Returns(Task.CompletedTask);
        return unitOfWork;
    }

    private static BackupRepository CreateRepository(BackupRepositorySpec spec)
        => new(
            name: "repo",
            description: null,
            spec: spec,
            passwordSecretId: Guid.CreateVersion7(),
            createdByActorId: Guid.CreateVersion7(),
            status: BackupRepositoryStatus.Ready);

    private static IUserContextAccessor CreateUserContextAccessor()
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.UserId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.ActorId).Returns(Guid.CreateVersion7());
        user.SetupGet(x => x.IsAdmin).Returns(true);
        user.SetupGet(x => x.IsAuthenticated).Returns(true);
        user.SetupGet(x => x.Roles).Returns(["admin"]);

        var accessor = new Mock<IUserContextAccessor>();
        accessor.SetupGet(x => x.Current).Returns(user.Object);
        return accessor.Object;
    }
}
