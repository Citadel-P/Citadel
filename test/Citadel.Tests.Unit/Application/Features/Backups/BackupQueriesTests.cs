using Application.Features.Backups.Queries;
using Application.Permissions;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Hosting.Common;
using Hosting.Common.Attributes;
using Moq;

namespace Tests.Unit.Application.Features.Backups;

public sealed class BackupQueriesTests
{
    [Fact]
    public async Task GetBackupRun_ShouldAuthorizeAgainstOwningPolicy()
    {
        var run = CreateRun();
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.GetAsync(run.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.BackupRuns).Returns(backupRuns.Object);
        var permissionEvaluator = new Mock<IPermissionEvaluator>();
        permissionEvaluator
            .Setup(x => x.EvaluateAsync(
                run.BackupPolicyId,
                ResourceType.BackupPolicy,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(PermissionMetadata.Empty);
        var handler = new GetBackupRunHandler(unitOfWork.Object, permissionEvaluator.Object);

        var result = await handler.Handle(
            new GetBackupRun(run.Id),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        permissionEvaluator.Verify(
            x => x.EvaluateAsync(
                run.BackupPolicyId,
                ResourceType.BackupPolicy,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task GetBackupRestoreRuns_ShouldRejectMismatchedPolicyAndRun()
    {
        var run = CreateRun();
        var backupRuns = new Mock<IBackupRunRepository>();
        backupRuns
            .Setup(x => x.GetAsync(run.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync(run);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.BackupRuns).Returns(backupRuns.Object);
        var permissionEvaluator = new Mock<IPermissionEvaluator>();
        var handler = new GetBackupRestoreRunsHandler(unitOfWork.Object, permissionEvaluator.Object);

        var result = await handler.Handle(
            new GetBackupRestoreRuns(run.Id, Guid.CreateVersion7()),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        permissionEvaluator.Verify(
            x => x.EvaluateAsync(
                It.IsAny<Guid>(),
                It.IsAny<ResourceType>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    private static BackupRun CreateRun()
        => new(
            backupPolicyId: Guid.CreateVersion7(),
            backupRepositoryId: Guid.CreateVersion7(),
            policyNameSnapshot: "policy",
            sourceSnapshot: new DockerVolumeBackupSource(Guid.CreateVersion7(), "data"),
            repositoryTypeSnapshot: BackupRepositoryType.FileSystem,
            trigger: BackupRunTrigger.Manual,
            triggerSourceId: null,
            triggeredByActorId: Guid.CreateVersion7());
}
