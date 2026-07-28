using Application.Features.Alerters.Commands;
using Application.Features.Backups.Queries;
using Application.Features.ResourceBindings.Commands;
using Citadel.SourceGen;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Pipelines.Interfaces;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class PermissionPipelineTests
{
    [Fact]
    public async Task Enforce_ShouldUseExplicitResourceIdProperty()
    {
        var policyId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var permissionService = CreatePermissionService(Helpers.AdminPermissions);

        var result = await PermissionPipeline.Enforce(
            new GetBackupPolicy(policyId),
            userId,
            permissionService.Object,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        permissionService.Verify(
            service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.BackupPolicy,
                policyId,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task Enforce_ShouldPreferDeclaredOwnerIdOverConventionalId()
    {
        var stackId = Guid.CreateVersion7();
        var bindingId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var permissionService = CreatePermissionService(Helpers.AdminPermissions);

        var result = await PermissionPipeline.Enforce(
            new DeleteStackResourceBinding(stackId, bindingId),
            userId,
            permissionService.Object,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        permissionService.Verify(
            service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.Stack,
                stackId,
                It.IsAny<CancellationToken>()),
            Times.Once);
        permissionService.Verify(
            service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.Stack,
                bindingId,
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task Enforce_ShouldNotTreatPlatformIdsAsBackupPolicyIds()
    {
        var platformId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var permissionService = CreatePermissionService(Helpers.AdminPermissions);

        var result = await PermissionPipeline.Enforce(
            new GetPlatformBackupSummaries([platformId]),
            userId,
            permissionService.Object,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
        permissionService.Verify(
            service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.BackupPolicy,
                null,
                It.IsAny<CancellationToken>()),
            Times.Once);
        permissionService.Verify(
            service => service.ResolvePermissionsAsyncForIds(
                It.IsAny<Guid>(),
                It.IsAny<ResourceType>(),
                It.IsAny<Guid[]>(),
                It.IsAny<CancellationToken>()),
            Times.Never);
    }

    [Fact]
    public async Task Enforce_ShouldRequireGlobalPermissionForEmptyIdCollection()
    {
        var permissionService = CreatePermissionService(PermissionMetadata.Empty);

        var result = await PermissionPipeline.Enforce(
            new DeleteAlertRules([]),
            Guid.CreateVersion7(),
            permissionService.Object,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
        permissionService.Verify(
            service => service.ResolvePermissionsAsync(
                It.IsAny<Guid>(),
                ResourceType.Alert,
                null,
                It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private static Mock<IPermissionService> CreatePermissionService(PermissionMetadata permission)
    {
        var service = new Mock<IPermissionService>();
        service
            .Setup(x => x.ResolvePermissionsAsync(
                It.IsAny<Guid>(),
                It.IsAny<ResourceType>(),
                It.IsAny<Guid?>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(permission);
        service
            .Setup(x => x.ResolvePermissionsAsyncForIds(
                It.IsAny<Guid>(),
                It.IsAny<ResourceType>(),
                It.IsAny<Guid[]>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Dictionary<Guid, PermissionMetadata>());
        return service;
    }
}
