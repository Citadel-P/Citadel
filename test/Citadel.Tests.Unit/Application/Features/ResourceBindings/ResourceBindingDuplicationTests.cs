using Application.Features.ResourceBindings;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.Pipelines.Interfaces;
using Moq;

namespace Tests.Unit.Application.Features.ResourceBindings;

public sealed class ResourceBindingDuplicationTests
{
    [Fact]
    public async Task GetDuplicateEntriesAsync_ShouldRequireSourceBindingPermission()
    {
        var sourceId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var uow = CreateUnitOfWork(sourceId);
        var permissions = new Mock<IPermissionService>();
        permissions.Setup(service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.SwarmService,
                sourceId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));

        var result = await ResourceBindingsFeatureHelpers.GetDuplicateEntriesAsync(
            uow.Object,
            permissions.Object,
            CreateUser(userId),
            ResourceType.SwarmService,
            ResourceBindingScope.SwarmService,
            sourceId,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("Resource Bindings", error.Message, StringComparison.Ordinal);
        permissions.Verify(service => service.ResolvePermissionsAsync(
            userId,
            ResourceType.SwarmService,
            null,
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public async Task GetDuplicateEntriesAsync_ShouldRequireTargetBindingPermission()
    {
        var sourceId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var uow = CreateUnitOfWork(sourceId);
        var permissions = new Mock<IPermissionService>();
        permissions.Setup(service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.SwarmService,
                sourceId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PermissionMetadata(PermissionLevel.Read, SpecificPermission.ResourceBindings));
        permissions.Setup(service => service.ResolvePermissionsAsync(
                userId,
                ResourceType.SwarmService,
                null,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new PermissionMetadata(PermissionLevel.Write, SpecificPermission.None));

        var result = await ResourceBindingsFeatureHelpers.GetDuplicateEntriesAsync(
            uow.Object,
            permissions.Object,
            CreateUser(userId),
            ResourceType.SwarmService,
            ResourceBindingScope.SwarmService,
            sourceId,
            TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Contains("Resource Bindings", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public async Task CopyDuplicateEntriesAsync_ShouldCreateNewBindingAndPreserveSecretReference()
    {
        var sourceId = Guid.CreateVersion7();
        var targetId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var source = new ResourceBinding(
            Name: "api_key",
            Kind: ResourceBindingKind.Secret,
            Scope: ResourceBindingScope.SwarmService,
            ResourceId: sourceId,
            Value: null,
            SecretId: secretId,
            SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);
        ResourceBinding[]? captured = null;
        var repository = new Mock<IResourceBindingRepository>();
        repository.Setup(value => value.ReplaceResourceEntriesAsync(
                ResourceBindingScope.SwarmService,
                targetId,
                It.IsAny<IEnumerable<ResourceBinding>>(),
                It.IsAny<CancellationToken>()))
            .Callback<ResourceBindingScope, Guid, IEnumerable<ResourceBinding>, CancellationToken>(
                (_, _, entries, _) => captured = entries.ToArray())
            .ReturnsAsync(1);
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(value => value.ResourceBindings).Returns(repository.Object);

        await ResourceBindingsFeatureHelpers.CopyDuplicateEntriesAsync(
            uow.Object,
            ResourceBindingScope.SwarmService,
            targetId,
            [source],
            TestContext.Current.CancellationToken);

        var copy = Assert.Single(Assert.IsType<ResourceBinding[]>(captured));
        Assert.NotEqual(source.Id, copy.Id);
        Assert.Equal(targetId, copy.ResourceId);
        Assert.Equal(secretId, copy.SecretId);
        Assert.Null(copy.Value);
    }

    private static Mock<IUnitOfWork> CreateUnitOfWork(Guid sourceId)
    {
        var repository = new Mock<IResourceBindingRepository>();
        repository.Setup(value => value.GetEntriesAsync(
                ResourceBindingScope.SwarmService,
                sourceId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync([
                new ResourceBinding(
                    Name: "environment",
                    Kind: ResourceBindingKind.Variable,
                    Scope: ResourceBindingScope.SwarmService,
                    ResourceId: sourceId,
                    Value: "production",
                    SecretId: null)
            ]);
        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(value => value.ResourceBindings).Returns(repository.Object);
        return uow;
    }

    private static IUserContext CreateUser(Guid userId)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(value => value.UserId).Returns(userId);
        user.SetupGet(value => value.ActorId).Returns(Guid.CreateVersion7());
        user.SetupGet(value => value.IsAdmin).Returns(false);
        return user.Object;
    }
}
