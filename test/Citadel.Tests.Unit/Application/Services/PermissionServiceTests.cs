using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.Caching.Memory;
using Moq;

namespace Tests.Unit.Application.Services;

public class PermissionServiceTests
{
    [Fact]
    public async Task HasPermissionAsync_Should_UseCachedValue_ForGlobalRequest()
    {
        var users = new Mock<IUserRepository>();
        users
            .Setup(x => x.HasPermissionAsync(It.IsAny<Guid>(), ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();

        var first = await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, CancellationToken.None);
        var second = await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, CancellationToken.None);

        Assert.True(first);
        Assert.True(second);
        users.Verify(x => x.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task HasPermissionAsync_Should_NotCache_ResourceScopedRequest()
    {
        var users = new Mock<IUserRepository>();
        users
            .Setup(x => x.HasPermissionAsync(It.IsAny<Guid>(), ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, It.IsAny<Guid?>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();
        var resourceId = Guid.NewGuid();

        await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, resourceId, CancellationToken.None);
        await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, resourceId, CancellationToken.None);

        users.Verify(x => x.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, resourceId, It.IsAny<CancellationToken>()), Times.Exactly(2));
    }

    [Fact]
    public async Task HasPermissionAsync_Should_NotReuseGlobalCache_ForDifferentPermissionShape()
    {
        var users = new Mock<IUserRepository>();
        users
            .Setup(x => x.HasPermissionAsync(It.IsAny<Guid>(), ResourceType.Deployment, It.IsAny<PermissionLevel>(), It.IsAny<SpecificPermission>(), null, It.IsAny<CancellationToken>()))
            .ReturnsAsync(false);

        var uow = new Mock<IUnitOfWork>();
        uow.SetupGet(x => x.Users).Returns(users.Object);

        using var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var service = new PermissionService(uow.Object, memoryCache);
        var userId = Guid.NewGuid();

        await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, CancellationToken.None);
        await service.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Execute, SpecificPermission.Apply, null, CancellationToken.None);

        users.Verify(x => x.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, null, It.IsAny<CancellationToken>()), Times.Once);
        users.Verify(x => x.HasPermissionAsync(userId, ResourceType.Deployment, PermissionLevel.Execute, SpecificPermission.Apply, null, It.IsAny<CancellationToken>()), Times.Once);
    }
}
