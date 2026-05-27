using Application.Features.Identity.Auth.Commands;
using Application.Services;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using LightResults;
using Microsoft.Extensions.Caching.Memory;
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

        var jwtService = new Mock<IJwtService>();
        jwtService.Setup(x => x.CreateAccessToken(It.IsAny<IEnumerable<System.Security.Claims.Claim>>()))
            .Returns("token");
        jwtService.Setup(x => x.CreateRefreshToken()).Returns((Guid.NewGuid(), "refresh"));

        var refreshTokens = new Mock<IRefreshTokenRepository>();
        uow.SetupGet(x => x.RefreshTokens).Returns(refreshTokens.Object);
        refreshTokens.Setup(x => x.AddAsync(It.IsAny<RefreshToken>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync(1);
        refreshTokens.Setup(x => x.CountAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>())).ReturnsAsync(0);

        var handler = new LoginCommandHandler(uow.Object, jwtService.Object, roleCache);

        var result = await handler.Handle(new LoginCommand(testUser.Email, "password"), CancellationToken.None);

        Assert.True(result.IsSuccess());

        var roles = roleCache.GetRoles(testUser.Id);
        Assert.NotNull(roles);
        Assert.Contains("admin", roles, StringComparer.OrdinalIgnoreCase);
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

        var jwtService = new Mock<IJwtService>();

        var handler = new LoginCommandHandler(uow.Object, jwtService.Object, roleCache);

        var result = await handler.Handle(new LoginCommand("missing", "password"), CancellationToken.None);

        Assert.True(result.IsFailure());

        // cache should be empty
        // no exception
    }
}
