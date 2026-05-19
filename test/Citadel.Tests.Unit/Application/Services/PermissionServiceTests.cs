using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.Caching.Memory;
using Moq;

namespace Tests.Unit.Application.Services;

public class PermissionServiceTests
{
    [Fact]
    public async Task HasPermissionAsync_CachesActorScope_WhenActorScopeNonEmpty()
    {
        var users = new Mock<IUserRepository>();
        var actorIds = new[] { Guid.NewGuid() };

        users
            .Setup(x => x.GetActorScopeAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(actorIds);

        users
            .Setup(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, (Guid?)null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Hosting.Common.Attributes.PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();

        var firstMeta = await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, CancellationToken.None);
        var secondMeta = await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, CancellationToken.None);

        var first = firstMeta.Has(PermissionLevel.Read, SpecificPermission.None);
        var second = secondMeta.Has(PermissionLevel.Read, SpecificPermission.None);

        Assert.True(first);
        Assert.True(second);

        users.Verify(x => x.GetActorScopeAsync(userId, It.IsAny<CancellationToken>()), Times.Once);
        users.Verify(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, null, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task HasPermissionAsync_Should_NotCache_ResourceScopedRequest()
    {
        var users = new Mock<IUserRepository>();
        var actorIds = new[] { Guid.NewGuid() };

        users
            .Setup(x => x.GetActorScopeAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(actorIds);
        users
            .Setup(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, It.IsAny<Guid?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Hosting.Common.Attributes.PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();
        var resourceId = Guid.NewGuid();

        await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, resourceId, CancellationToken.None);
        await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, resourceId, CancellationToken.None);

        users.Verify(x => x.GetActorScopeAsync(userId, It.IsAny<CancellationToken>()), Times.Once);
        users.Verify(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, resourceId, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task HasPermissionAsync_CallsHasPermission_ForDifferentPermissionShapes()
    {
        var users = new Mock<IUserRepository>();
        var actorIds = new[] { Guid.NewGuid() };

        users
            .Setup(x => x.GetActorScopeAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(actorIds);
        users
            .Setup(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, (Guid?)null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new Hosting.Common.Attributes.PermissionMetadata(PermissionLevel.Read, SpecificPermission.None));

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();

        await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, CancellationToken.None);
        await service.ResolvePermissionsAsync(userId, ResourceType.Deployment, null, CancellationToken.None);

        users.Verify(x => x.GetActorScopeAsync(userId, It.IsAny<CancellationToken>()), Times.Once);
        users.Verify(x => x.GetEffectivePermissionsAsync(It.IsAny<Guid[]>(), ResourceType.Deployment, null, It.IsAny<CancellationToken>()), Times.Once);
    }
}
