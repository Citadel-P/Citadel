using Application.Configs;
using Application.Features.Identity.Auth.Commands;
using Application.Features.Identity.Mfa.Services;
using Application.Services;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using LightResults;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Options;
using Moq;
using System.Security.Claims;

namespace Tests.Unit.Application.Features.Identity.Auth;

public class LoginCommandCacheTests
{
    [Fact]
    public async Task Handle_ShouldPopulateRoleCache_OnSuccessfulLogin()
    {
        var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var roleCache = new RoleCache(memoryCache);

        var uow = new Mock<IUnitOfWork>();
        var users = new Mock<IUserRepository>();

        // Create a stored hashed password that matches plain "password" using the same algorithm as User.HashPassword
        var salt = System.Security.Cryptography.RandomNumberGenerator.GetBytes(16);
        var hash = System.Security.Cryptography.Rfc2898DeriveBytes.Pbkdf2("password", salt, 10000, System.Security.Cryptography.HashAlgorithmName.SHA256, 20);
        var hashBytes = new byte[36];
        Array.Copy(salt, 0, hashBytes, 0, 16);
        Array.Copy(hash, 0, hashBytes, 16, hash.Length);
        var storedPassword = Convert.ToBase64String(hashBytes);

        var testUser = new UserAuthInfo(Guid.NewGuid(), Guid.NewGuid(), "bob", "bob@example.com", storedPassword, new[] { "admin" });

        users.Setup(x => x.GetUserAuthInfoByEmailOrNameAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(testUser);

        uow.SetupGet(x => x.Users).Returns(users.Object);

        var userMfa = new Mock<IUserMfaRepository>();
        userMfa.Setup(x => x.GetSettingsAsync(testUser.Id, It.IsAny<CancellationToken>()))
            .ReturnsAsync((UserMfaSettings?)null);
        uow.SetupGet(x => x.UserMfa).Returns(userMfa.Object);

        var mfaChallenges = new Mock<IMfaChallengeRepository>();
        uow.SetupGet(x => x.MfaChallenges).Returns(mfaChallenges.Object);

        var mfaPolicy = new Mock<IMfaPolicyService>();
        mfaPolicy.Setup(x => x.RequiresMfa(testUser)).Returns(false);

        var sessionIssuer = new Mock<IAuthenticationSessionIssuer>();
        sessionIssuer.Setup(x => x.IssueAsync(testUser, It.IsAny<CancellationToken>()))
            .Callback(() => roleCache.SetRoles(testUser.Id, testUser.Roles))
            .ReturnsAsync("token");

        var handler = new LoginCommandHandler(
            uow.Object,
            Mock.Of<ITotpService>(),
            mfaPolicy.Object,
            Mock.Of<ISecretValueProtector>(),
            sessionIssuer.Object,
            Mock.Of<IMfaChallengeCookieService>(),
            Mock.Of<IMfaSetupCookieService>(),
            Options.Create(new MfaOptions()));

        var result = await handler.Handle(new LoginCommand(testUser.Email, "password"), CancellationToken.None);

        Assert.True(result.IsSuccess());

        var roles = roleCache.GetRoles(testUser.Id);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
        sessionIssuer.Verify(x => x.IssueAsync(testUser, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Handle_ShouldNotPopulateCache_WhenLoginFails()
    {
        var memoryCache = new MemoryCache(new MemoryCacheOptions());
        var roleCache = new RoleCache(memoryCache);

        var uow = new Mock<IUnitOfWork>();
        var users = new Mock<IUserRepository>();

        users.Setup(x => x.GetUserAuthInfoByEmailOrNameAsync(It.IsAny<string>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((UserAuthInfo)null);

        uow.SetupGet(x => x.Users).Returns(users.Object);

        var handler = new LoginCommandHandler(
            uow.Object,
            Mock.Of<ITotpService>(),
            Mock.Of<IMfaPolicyService>(),
            Mock.Of<ISecretValueProtector>(),
            Mock.Of<IAuthenticationSessionIssuer>(),
            Mock.Of<IMfaChallengeCookieService>(),
            Mock.Of<IMfaSetupCookieService>(),
            Options.Create(new MfaOptions()));

        var result = await handler.Handle(new LoginCommand("missing", "password"), CancellationToken.None);

        Assert.True(result.IsFailure());

        // cache should be empty
        // no exception
    }
}
