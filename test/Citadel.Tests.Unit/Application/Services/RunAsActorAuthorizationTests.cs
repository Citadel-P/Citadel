using Application.Services;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class RunAsActorAuthorizationTests
{
    [Fact]
    public async Task NonAdministrator_Can_Run_As_Self()
    {
        var actorId = Guid.CreateVersion7();
        var sut = CreateSut(CreateUser(actorId, isAdmin: false), User(actorId));

        var result = await sut.EnsureAllowedAsync(actorId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
    }

    [Fact]
    public async Task NonAdministrator_Cannot_Run_As_Another_User()
    {
        var targetId = Guid.CreateVersion7();
        var sut = CreateSut(CreateUser(Guid.CreateVersion7(), isAdmin: false), User(targetId));

        var result = await sut.EnsureAllowedAsync(targetId, TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure());
    }

    [Fact]
    public async Task Administrator_Can_Run_As_Another_User()
    {
        var targetId = Guid.CreateVersion7();
        var sut = CreateSut(CreateUser(Guid.CreateVersion7(), isAdmin: true), User(targetId));

        var result = await sut.EnsureAllowedAsync(targetId, TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess());
    }

    [Fact]
    public async Task Delegated_User_Needs_Use_Permission_For_Service_Account()
    {
        var targetId = Guid.CreateVersion7();
        var denied = CreateSut(
            CreateUser(Guid.CreateVersion7(), isAdmin: false),
            ServiceAccount(targetId),
            PermissionMetadata.Empty);
        var allowed = CreateSut(
            CreateUser(Guid.CreateVersion7(), isAdmin: false),
            ServiceAccount(targetId),
            new PermissionMetadata(PermissionLevel.Read, SpecificPermission.Use));

        var deniedResult = await denied.EnsureAllowedAsync(targetId, TestContext.Current.CancellationToken);
        var allowedResult = await allowed.EnsureAllowedAsync(targetId, TestContext.Current.CancellationToken);

        Assert.True(deniedResult.IsFailure());
        Assert.True(allowedResult.IsSuccess());
    }

    private static RunAsActorAuthorization CreateSut(
        IUserContext current,
        RunAsActorInfo target,
        PermissionMetadata permissions = default)
    {
        var actors = new Mock<IActorRepository>();
        actors.Setup(x => x.GetRunAsInfoAsync(target.ActorId, It.IsAny<CancellationToken>())).ReturnsAsync(target);
        var unitOfWork = new Mock<IUnitOfWork>();
        unitOfWork.SetupGet(x => x.Actors).Returns(actors.Object);
        var accessor = new Mock<IUserContextAccessor>();
        accessor.SetupGet(x => x.Current).Returns(current);
        var permissionService = new Mock<IPermissionService>();
        permissionService
            .Setup(x => x.ResolvePermissionsAsync(
                It.IsAny<Guid>(),
                ResourceType.ServiceAccount,
                target.PrincipalId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(permissions);
        var entitlement = new Mock<ILicenseEntitlementService>();
        entitlement
            .Setup(x => x.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Success());
        return new RunAsActorAuthorization(unitOfWork.Object, accessor.Object, permissionService.Object, entitlement.Object);
    }

    private static RunAsActorInfo User(Guid actorId)
        => new(actorId, Guid.CreateVersion7(), ActorType.User, "user", true, false, []);

    private static RunAsActorInfo ServiceAccount(Guid actorId)
        => new(actorId, Guid.CreateVersion7(), ActorType.ServiceAccount, "service-account", true, false, []);

    private static IUserContext CreateUser(Guid actorId, bool isAdmin)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.ActorId).Returns(actorId);
        user.SetupGet(x => x.IsAdmin).Returns(isAdmin);
        return user.Object;
    }
}
