using Application.Services;
using Hosting.Common.Abstraction;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class RunAsActorAuthorizationTests
{
    [Fact]
    public void NonAdministrator_Can_Run_As_Self()
    {
        var actorId = Guid.CreateVersion7();

        var result = RunAsActorAuthorization.EnsureAllowed(CreateUser(actorId, isAdmin: false), actorId);

        Assert.True(result.IsSuccess());
    }

    [Fact]
    public void NonAdministrator_Cannot_Run_As_Another_User()
    {
        var result = RunAsActorAuthorization.EnsureAllowed(
            CreateUser(Guid.CreateVersion7(), isAdmin: false),
            Guid.CreateVersion7());

        Assert.True(result.IsFailure());
    }

    [Fact]
    public void Administrator_Can_Run_As_Another_User()
    {
        var result = RunAsActorAuthorization.EnsureAllowed(
            CreateUser(Guid.CreateVersion7(), isAdmin: true),
            Guid.CreateVersion7());

        Assert.True(result.IsSuccess());
    }

    private static IUserContext CreateUser(Guid actorId, bool isAdmin)
    {
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.ActorId).Returns(actorId);
        user.SetupGet(x => x.IsAdmin).Returns(isAdmin);
        return user.Object;
    }
}
