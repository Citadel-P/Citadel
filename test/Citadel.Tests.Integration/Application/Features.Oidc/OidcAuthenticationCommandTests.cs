using Application.Features.Oidc.Commands;
using Application.Services;
using Application.Services.Identity;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Oidc;
using Domain.Entities.Oidc;
using Hosting.Common;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using Tests.Integration.Helpers;
using static Application.Features.Oidc.Commands.OidcOpaqueValue;

namespace Tests.Integration.Application.Features.Oidc;

public sealed class OidcAuthenticationCommandTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid SeededAdminUserId = Guid.Parse("10000000-0000-0000-0000-000000000001");
    private readonly FakeOidcDiscoveryService discoveryService = new();
    private readonly FakeOidcAuthenticationService authenticationService = new();

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.ReplaceService<IOidcDiscoveryService>(discoveryService);
        services.ReplaceService<IOidcAuthenticationService>(authenticationService);
    }

    [Fact]
    public async Task BeginOidcLogin_ShouldPersistStateAndReturnAuthorizationUrl()
    {
        await using var scope = Services.CreateAsyncScope();
        var provider = await CreateProviderAsync(scope.ServiceProvider, "begin-oidc", allowEmailAutoLink: true);
        var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();

        var result = await mediator.Send(
            new BeginOidcLogin(provider.Id, CallbackUri(provider.Id), "https://frontend.test/login", ["https://frontend.test"]),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var start), string.Join(Environment.NewLine, result.Errors.Select(x => x.Message)));
        Assert.StartsWith("https://issuer.test/authorize?", start.AuthorizationUrl, StringComparison.Ordinal);

        var authorizationUri = new Uri(start.AuthorizationUrl);
        var query = ParseQuery(authorizationUri.Query);
        Assert.Equal(provider.ClientId, query["client_id"]);
        Assert.Equal(CallbackUri(provider.Id), query["redirect_uri"]);
        Assert.Equal("https://frontend.test/login", query["return_url"]);
        Assert.False(string.IsNullOrWhiteSpace(query["state"]));

        var persistedState = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .OidcLoginStates
            .GetByStateHashAsync(HashOpaqueValue(query["state"]), TestContext.Current.CancellationToken);

        Assert.NotNull(persistedState);
        Assert.Equal(provider.Id, persistedState.ProviderId);
        Assert.Equal("https://frontend.test/login", persistedState.ReturnUrl);
    }

    [Fact]
    public async Task BeginOidcLogin_ShouldRejectUntrustedAbsoluteReturnUrl()
    {
        await using var scope = Services.CreateAsyncScope();
        var provider = await CreateProviderAsync(scope.ServiceProvider, "reject-return-url", allowEmailAutoLink: true);
        var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();

        var result = await mediator.Send(
            new BeginOidcLogin(provider.Id, CallbackUri(provider.Id), "https://evil.test/login", ["https://frontend.test"]),
            TestContext.Current.CancellationToken);

        Assert.False(result.IsSuccess());
        Assert.Contains(result.Errors, error => error.Message == "Return URL is invalid.");
    }

    [Fact]
    public async Task CompleteOidcLogin_ShouldAutoLinkExistingUserAndPersistExternalLogin()
    {
        await using var scope = Services.CreateAsyncScope();
        var provider = await CreateProviderAsync(scope.ServiceProvider, "link-oidc", allowEmailAutoLink: true);
        await AddLoginStateAsync(scope.ServiceProvider, provider.Id, "link-state", "https://frontend.test/login");
        authenticationService.Identity = new OidcTokenIdentity(
            "github|admin",
            "admin@citadel.local",
            EmailVerified: true,
            "Citadel Admin",
            []);

        var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();
        var result = await mediator.Send(
            new CompleteOidcLogin(provider.Id, "code", "link-state", CallbackUri(provider.Id)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var complete), string.Join(Environment.NewLine, result.Errors.Select(x => x.Message)));
        Assert.Equal("https://frontend.test/login", complete.ReturnUrl);
        Assert.False(string.IsNullOrWhiteSpace(complete.Login.AccessToken));

        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var externalLogin = await uow.OidcExternalLogins.GetAsync(provider.Id, "github|admin", TestContext.Current.CancellationToken);
        Assert.NotNull(externalLogin);
        Assert.Equal(SeededAdminUserId, externalLogin.UserId);

        var linkedState = await uow.OidcLoginStates.GetByStateHashAsync(HashOpaqueValue("link-state"), TestContext.Current.CancellationToken);
        Assert.Null(linkedState);

        var jwt = new JwtSecurityTokenHandler().ReadJwtToken(complete.Login.AccessToken);
        Assert.Equal(SeededAdminUserId.ToString(), jwt.Claims.Single(x => x.Type == JwtRegisteredClaimNames.Sub).Value);
        Assert.Contains("Admin", scope.ServiceProvider.GetRequiredService<IRoleCache>().GetRoles(SeededAdminUserId)!);
    }

    [Fact]
    public async Task CompleteOidcLogin_ShouldAutoProvisionUserWithDefaultRole()
    {
        await using var scope = Services.CreateAsyncScope();
        var provider = await CreateProviderAsync(
            scope.ServiceProvider,
            "provision-oidc",
            autoProvisionUsers: true,
            defaultRoleId: ViewerRoleId);
        await AddLoginStateAsync(scope.ServiceProvider, provider.Id, "provision-state", "/login");
        authenticationService.Identity = new OidcTokenIdentity(
            "github|new-user",
            "new-user@example.com",
            EmailVerified: true,
            "new user",
            []);

        var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();
        var result = await mediator.Send(
            new CompleteOidcLogin(provider.Id, "code", "provision-state", CallbackUri(provider.Id)),
            TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var complete), string.Join(Environment.NewLine, result.Errors.Select(x => x.Message)));
        Assert.Equal("/login", complete.ReturnUrl);

        var jwt = new JwtSecurityTokenHandler().ReadJwtToken(complete.Login.AccessToken);
        var userId = Guid.Parse(jwt.Claims.Single(x => x.Type == JwtRegisteredClaimNames.Sub).Value);

        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var provisioned = await uow.Users.GetUserAuthInfoByIdAsync(userId, TestContext.Current.CancellationToken);
        Assert.NotNull(provisioned);
        Assert.Equal("new-user@example.com", provisioned.Email);
        Assert.Contains("viewer", provisioned.Roles, StringComparer.OrdinalIgnoreCase);

        var assignedRoles = await uow.Roles.GetActorRoleIdsAsync(provisioned.ActorId, TestContext.Current.CancellationToken);
        Assert.Contains(ViewerRoleId, assignedRoles);

        var externalLogin = await uow.OidcExternalLogins.GetAsync(provider.Id, "github|new-user", TestContext.Current.CancellationToken);
        Assert.NotNull(externalLogin);
        Assert.Equal(userId, externalLogin.UserId);
    }

    private static async Task<OidcProvider> CreateProviderAsync(
        IServiceProvider services,
        string name,
        bool autoProvisionUsers = false,
        bool allowEmailAutoLink = false,
        Guid? defaultRoleId = null)
    {
        var provider = new OidcProvider(
            name,
            null,
            name,
            "https://issuer.test",
            "client-id",
            null,
            "openid profile email",
            enabled: true,
            autoProvisionUsers,
            allowEmailAutoLink,
            requireEmailVerified: true,
            allowedEmailDomains: null,
            requiredClaimName: null,
            requiredClaimValues: null,
            defaultRoleId,
            Constants.SystemId);

        var uow = services.GetRequiredService<IUnitOfWork>();
        await uow.OidcProviders.AddAsync(provider, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return provider;
    }

    private static async Task AddLoginStateAsync(IServiceProvider services, Guid providerId, string state, string returnUrl)
    {
        var now = DateTime.UtcNow;
        var uow = services.GetRequiredService<IUnitOfWork>();
        await uow.OidcLoginStates.AddAsync(
            new OidcLoginState(
                Guid.CreateVersion7(),
                providerId,
                HashOpaqueValue(state),
                "nonce",
                "code-verifier",
                returnUrl,
                now,
                now.AddMinutes(10)),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    private static string CallbackUri(Guid providerId)
        => $"https://citadel.test/api/v1/authentication/oidc/{providerId}/callback";

    private static Dictionary<string, string> ParseQuery(string query)
        => query.TrimStart('?')
            .Split('&', StringSplitOptions.RemoveEmptyEntries)
            .Select(part => part.Split('=', 2))
            .ToDictionary(
                parts => Uri.UnescapeDataString(parts[0]),
                parts => parts.Length > 1 ? Uri.UnescapeDataString(parts[1]) : string.Empty);

    private sealed class FakeOidcDiscoveryService : IOidcDiscoveryService
    {
        private static readonly OidcDiscoveryResult Discovery = new(
            "https://issuer.test",
            "https://issuer.test/authorize",
            "https://issuer.test/token",
            "https://issuer.test/jwks");

        public Task<Result<OidcDiscoveryResult>> GetDiscoveryAsync(string issuer, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(Discovery));

        public Task<Result<OidcDiscoveryResult>> TestDiscoveryAsync(string issuer, CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(Discovery));
    }

    private sealed class FakeOidcAuthenticationService : IOidcAuthenticationService
    {
        public OidcTokenIdentity Identity { get; set; } = new(
            "subject",
            "admin@citadel.local",
            EmailVerified: true,
            "Admin",
            []);

        public OidcAuthorizationRequest CreateAuthorizationRequest(
            OidcProvider provider,
            OidcDiscoveryResult discovery,
            string redirectUri,
            string state,
            string nonce,
            string codeVerifier)
        {
            return new OidcAuthorizationRequest(
                $"{discovery.AuthorizationEndpoint}?client_id={Uri.EscapeDataString(provider.ClientId)}&redirect_uri={Uri.EscapeDataString(redirectUri)}&return_url={Uri.EscapeDataString("https://frontend.test/login")}&state={Uri.EscapeDataString(state)}",
                codeVerifier);
        }

        public Task<Result<OidcTokenIdentity>> ExchangeAndValidateAsync(
            OidcProvider provider,
            OidcDiscoveryResult discovery,
            string clientSecret,
            string code,
            string redirectUri,
            string codeVerifier,
            string nonce,
            CancellationToken cancellationToken)
            => Task.FromResult(Result.Success(Identity));
    }
}
