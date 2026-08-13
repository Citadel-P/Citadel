using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Attributes;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using WebApi.Security;

namespace Tests.Integration.Application.Features.Identity.ServiceAccounts;

public sealed class ServiceAccountEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override void ConfigureTestServices(IServiceCollection services)
        => services.AddSingleton<IHostedService>(provider =>
            provider.GetRequiredService<ServiceAccountLastUsedTracker>());

    [Fact]
    public async Task Create_ShouldPersistAccountWithoutImplicitAccess()
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/serviceAccounts/",
            new
            {
                name = "ci-readonly",
                description = "Used by CI",
                isEnabled = true,
                teamIds = Array.Empty<Guid>(),
                roleIds = Array.Empty<Guid>(),
                resourceAccesses = Array.Empty<object>(),
            },
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var json = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        var accountId = json.RootElement.GetProperty("id").GetGuid();
        var actorId = json.RootElement.GetProperty("actorId").GetGuid();

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var account = await unitOfWork.ServiceAccounts.GetAsync(accountId, TestContext.Current.CancellationToken);
        var actor = await unitOfWork.Actors.GetById(actorId, TestContext.Current.CancellationToken);

        Assert.NotNull(account);
        Assert.Equal("ci-readonly", account.Name);
        Assert.Equal(ActorType.ServiceAccount, actor?.Type);
        Assert.True(actor?.IsEnabled);
        Assert.Empty(await unitOfWork.ServiceAccounts.GetTeamIdsAsync(actorId, TestContext.Current.CancellationToken));
        Assert.Empty(await unitOfWork.Roles.GetActorRoleIdsAsync(actorId, TestContext.Current.CancellationToken));
        Assert.Empty(await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(actorId, TestContext.Current.CancellationToken));

        using var getResponse = await Client.GetAsync(
            $"/api/v1/serviceAccounts/{accountId}",
            TestContext.Current.CancellationToken);
        getResponse.EnsureSuccessStatusCode();
        using var getJson = JsonDocument.Parse(
            await getResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        Assert.True(getJson.RootElement.GetProperty("capabilities").GetProperty("canManageCredentials").GetBoolean());
    }

    [Fact]
    public async Task Token_ShouldAuthenticateWithInheritedPermissionsAndRespectLifecycle()
    {
        var teamActor = Actor.Create(ActorType.Team, new ActorMetadata("service-account-viewers"));
        var team = Team.Create("service-account-viewers", teamActor.Id);
        var platform = new Platform(
            name: "service-account-visible-platform",
            address: "https://service-account-visible.invalid",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor("daemon-service-account", 0, 0, 0, 0));
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await unitOfWork.Actors.AddAsync(teamActor, TestContext.Current.CancellationToken);
            await unitOfWork.Teams.AddAsync(team, TestContext.Current.CancellationToken);
            await unitOfWork.Roles.AddActorRoleAsync(teamActor.Id, ViewerRoleId, TestContext.Current.CancellationToken);
            await unitOfWork.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        var adminAuthorization = Client.DefaultRequestHeaders.Authorization;
        var account = await CreateAccountAsync("ci-team-reader", [team.Id]);
        var createdToken = await CreateTokenAsync(account.Id, "pipeline", expiresAtUtc: null);

        Assert.StartsWith("cit_sa_", createdToken.Token, StringComparison.Ordinal);
        Assert.Equal("no-store", createdToken.CacheControl);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", createdToken.Token);
        var allowed = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.OK, allowed.StatusCode);
        using (var allowedJson = JsonDocument.Parse(
                   await allowed.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken)))
        {
            Assert.Contains(
                allowedJson.RootElement.GetProperty("platforms").EnumerateArray(),
                item => item.GetProperty("id").GetGuid() == platform.Id);
        }

        var humanOnly = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            new { name = "forbidden", expiresAtUtc = (DateTime?)null },
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Forbidden, humanOnly.StatusCode);

        Client.DefaultRequestHeaders.Authorization = adminAuthorization;
        await UpdateEnabledAsync(account, isEnabled: false);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", createdToken.Token);
        var disabled = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Unauthorized, disabled.StatusCode);

        Client.DefaultRequestHeaders.Authorization = adminAuthorization;
        await UpdateEnabledAsync(account, isEnabled: true);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", createdToken.Token);
        var reEnabled = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.OK, reEnabled.StatusCode);

        Client.DefaultRequestHeaders.Authorization = adminAuthorization;
        var revoke = await Client.DeleteAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens/{createdToken.CredentialId}",
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, revoke.StatusCode);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", createdToken.Token);
        var revoked = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Unauthorized, revoked.StatusCode);
    }

    [Fact]
    public async Task Token_ShouldBeReturnedOnceAndPersistOnlyItsDigest()
    {
        var account = await CreateAccountAsync("ci-token-storage", []);
        var created = await CreateTokenAsync(account.Id, "non-expiring", expiresAtUtc: null);

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await unitOfWork.ServiceAccounts.GetCredentialAsync(
            created.CredentialId,
            TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(32, persisted.SecretHash.Length);
        Assert.Null(persisted.ExpiresAtUtc);

        var history = await Client.GetStringAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            TestContext.Current.CancellationToken);
        Assert.DoesNotContain(created.Token, history, StringComparison.Ordinal);
        Assert.DoesNotContain("secretHash", history, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("cit_sa_", history, StringComparison.Ordinal);
    }

    [Fact]
    public async Task TokenAuthentication_ShouldPersistLastUsedMetadata()
    {
        var account = await CreateAccountAsync("ci-last-used", []);
        var created = await CreateTokenAsync(account.Id, "observed", expiresAtUtc: null);
        var adminAuthorization = Client.DefaultRequestHeaders.Authorization;

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", created.Token);
        var authenticated = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.OK, authenticated.StatusCode);
        Client.DefaultRequestHeaders.Authorization = adminAuthorization;

        DateTime? lastUsedAtUtc = null;
        for (var attempt = 0; attempt < 50 && lastUsedAtUtc is null; attempt++)
        {
            await using var scope = Services.CreateAsyncScope();
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var tokens = await unitOfWork.ServiceAccounts.GetTokensAsync(
                account.Id,
                page: 1,
                pageSize: 10,
                TestContext.Current.CancellationToken);
            lastUsedAtUtc = tokens.Items.Single(token => token.Id == created.CredentialId).LastUsedAtUtc;
            if (lastUsedAtUtc is null)
                await Task.Delay(50, TestContext.Current.CancellationToken);
        }

        Assert.NotNull(lastUsedAtUtc);
    }

    [Fact]
    public async Task Token_ShouldUseConfiguredDefaultUnlessNeverIsExplicit()
    {
        var account = await CreateAccountAsync("ci-token-expiration", []);

        var defaultResponse = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            new { name = "default-lifetime" },
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Created, defaultResponse.StatusCode);
        using (var json = JsonDocument.Parse(
                   await defaultResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken)))
        {
            Assert.NotEqual(
                JsonValueKind.Null,
                json.RootElement.GetProperty("expiresAtUtc").ValueKind);
        }

        var neverResponse = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            new { name = "explicit-never", neverExpires = true },
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Created, neverResponse.StatusCode);
        using var neverJson = JsonDocument.Parse(
            await neverResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        Assert.False(neverJson.RootElement.TryGetProperty("expiresAtUtc", out _));
    }

    [Fact]
    public async Task Token_WithOffsetExpiration_ShouldPersistTheUtcInstant()
    {
        var account = await CreateAccountAsync("ci-token-offset", []);
        var requestedExpiration = DateTimeOffset.UtcNow
            .AddDays(1)
            .ToOffset(TimeSpan.FromHours(2));

        var response = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            new { name = "offset-expiration", expiresAtUtc = requestedExpiration },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Created, response.StatusCode);
        using var json = JsonDocument.Parse(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        var tokenId = json.RootElement.GetProperty("id").GetGuid();

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await unitOfWork.ServiceAccounts.GetCredentialAsync(
            tokenId,
            TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(
            requestedExpiration.UtcDateTime,
            persisted.ExpiresAtUtc!.Value,
            TimeSpan.FromMicroseconds(1));
        Assert.Equal(DateTimeKind.Utc, persisted.ExpiresAtUtc?.Kind);
    }

    [Fact]
    public async Task ActivityView_ShouldResolveServiceAccountActorName()
    {
        var account = await CreateAccountAsync("ci-activity-actor", []);
        Guid actorId;
        Guid activityId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var persisted = await unitOfWork.ServiceAccounts.GetAsync(account.Id, TestContext.Current.CancellationToken);
            Assert.NotNull(persisted);
            actorId = persisted.ActorId;
            var activity = new ActivityEvent(
                platformId: null,
                resourceId: account.Id,
                actorId: actorId,
                resourceName: account.Name,
                eventType: ActivityEventType.ServiceAccountTokenRevoked,
                status: ActivityStatus.Success,
                info: new ServiceAccountTokenRevoked(account.Id, Guid.CreateVersion7(), "cit_sa_test"));
            activityId = activity.Id;
            await unitOfWork.ActivityEventRepository.AddAsync(activity, TestContext.Current.CancellationToken);
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        using var response = await Client.GetAsync(
            $"/api/v1/activities/{activityId}",
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();
        using var json = JsonDocument.Parse(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        Assert.Equal(actorId, json.RootElement.GetProperty("actorId").GetGuid());
        Assert.Equal("ci-activity-actor", json.RootElement.GetProperty("actorName").GetString());
        Assert.Equal("ServiceAccount", json.RootElement.GetProperty("actorType").GetString());
    }

    [Fact]
    public async Task Token_ShouldNotBeIssuedWhileAccountIsDisabled()
    {
        var account = await CreateAccountAsync("ci-disabled-token", []);
        await UpdateEnabledAsync(account, isEnabled: false);

        var response = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/tokens",
            new { name = "blocked", neverExpires = true },
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task ResourceScopedManageCredentials_ShouldApplyOnlyToTheTargetAccount()
    {
        var allowedAccount = await CreateAccountAsync("ci-delegated-token", []);
        var deniedAccount = await CreateAccountAsync("ci-not-delegated-token", []);
        var subject = await CreateAuthorizationSubjectAsync(resourceGrants:
        [
            new ResourceGrant(
                ResourceType.ServiceAccount,
                allowedAccount.Id,
                PermissionLevel.Read,
                SpecificPermission.ManageCredentials),
        ]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        Assert.Equal(
            HttpStatusCode.OK,
            (await Client.GetAsync(
                $"/api/v1/serviceAccounts/{allowedAccount.Id}",
                TestContext.Current.CancellationToken)).StatusCode);
        Assert.Equal(
            HttpStatusCode.Forbidden,
            (await Client.GetAsync(
                $"/api/v1/serviceAccounts/{deniedAccount.Id}",
                TestContext.Current.CancellationToken)).StatusCode);

        Assert.Equal(
            HttpStatusCode.Created,
            (await Client.PostAsJsonAsync(
                $"/api/v1/serviceAccounts/{allowedAccount.Id}/tokens",
                new { name = "delegated", neverExpires = true },
                TestContext.Current.CancellationToken)).StatusCode);
        Assert.Equal(
            HttpStatusCode.Forbidden,
            (await Client.PostAsJsonAsync(
                $"/api/v1/serviceAccounts/{deniedAccount.Id}/tokens",
                new { name = "denied", neverExpires = true },
                TestContext.Current.CancellationToken)).StatusCode);
    }

    [Fact]
    public async Task InvalidTokens_ShouldReturnGenericBearerChallenge()
    {
        var invalidTokens = new[]
        {
            "cit_sa_",
            $"cit_sa_{Guid.NewGuid():N}.invalid",
            $"cit_sa_{Guid.NewGuid():N}.{new string('A', 43)}",
        };

        foreach (var token in invalidTokens)
        {
            Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
            var response = await Client.GetAsync("/api/v1/platforms/", TestContext.Current.CancellationToken);
            var body = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

            Assert.Equal(HttpStatusCode.Unauthorized, response.StatusCode);
            Assert.Contains(response.Headers.WwwAuthenticate, header => header.Scheme == "Bearer");
            Assert.DoesNotContain("credential", body, StringComparison.OrdinalIgnoreCase);
            Assert.DoesNotContain("token", body, StringComparison.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public async Task ConcurrentNames_ShouldUseTheExistingCaseSensitiveIdentityConvention()
    {
        var first = Client.PostAsJsonAsync(
            "/api/v1/serviceAccounts/",
            new { name = "Concurrent-CI", isEnabled = true },
            TestContext.Current.CancellationToken);
        var second = Client.PostAsJsonAsync(
            "/api/v1/serviceAccounts/",
            new { name = "concurrent-ci", isEnabled = true },
            TestContext.Current.CancellationToken);

        var responses = await Task.WhenAll(first, second);

        Assert.All(responses, response => Assert.True(response.IsSuccessStatusCode));
    }

    [Fact]
    public async Task ConcurrentExactNames_ShouldReturnOneSuccessAndOneConflict()
    {
        var payload = new { name = "concurrent-exact-ci", isEnabled = true };
        var responses = await Task.WhenAll(
            Client.PostAsJsonAsync(
                "/api/v1/serviceAccounts/",
                payload,
                TestContext.Current.CancellationToken),
            Client.PostAsJsonAsync(
                "/api/v1/serviceAccounts/",
                payload,
                TestContext.Current.CancellationToken));

        Assert.Single(responses, response => response.IsSuccessStatusCode);
        Assert.Single(responses, response => response.StatusCode == HttpStatusCode.Conflict);
    }

    [Fact]
    public async Task ReplacingTeamUsers_ShouldPreserveServiceAccountMemberships()
    {
        var teamActor = Actor.Create(ActorType.Team, new ActorMetadata("mixed-members"));
        var team = Team.Create("mixed-members", teamActor.Id);
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await unitOfWork.Actors.AddAsync(teamActor, TestContext.Current.CancellationToken);
            await unitOfWork.Teams.AddAsync(team, TestContext.Current.CancellationToken);
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        var account = await CreateAccountAsync("mixed-member-service", [team.Id]);

        using (var teamJson = JsonDocument.Parse(
                   await Client.GetStreamAsync($"/api/v1/teams/{team.Id}", TestContext.Current.CancellationToken)))
        {
            Assert.Equal(1, teamJson.RootElement.GetProperty("totalMembers").GetInt32());
            Assert.Empty(teamJson.RootElement.GetProperty("users").EnumerateArray());
            var member = Assert.Single(teamJson.RootElement.GetProperty("members").EnumerateArray());
            Assert.Equal(account.Id, member.GetProperty("resourceId").GetGuid());
            Assert.Equal("ServiceAccount", member.GetProperty("principalType").GetString());
        }

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUnitOfWork = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await verificationUnitOfWork.Teams.ReplaceMembersAsync(team.Id, [], TestContext.Current.CancellationToken);
        await verificationUnitOfWork.CommitAsync(TestContext.Current.CancellationToken);

        var persistedAccount = await verificationUnitOfWork.ServiceAccounts.GetAsync(
            account.Id,
            TestContext.Current.CancellationToken);
        Assert.NotNull(persistedAccount);
        Assert.Contains(
            team.Id,
            await verificationUnitOfWork.ServiceAccounts.GetTeamIdsAsync(
                persistedAccount.ActorId,
                TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task StandardMutationEndpoints_ShouldPersistAssignmentsAndArchiveAtomically()
    {
        var teamActor = Actor.Create(ActorType.Team, new ActorMetadata("api-members"));
        var team = Team.Create("api-members", teamActor.Id);
        Guid roleId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await unitOfWork.Actors.AddAsync(teamActor, TestContext.Current.CancellationToken);
            await unitOfWork.Teams.AddAsync(team, TestContext.Current.CancellationToken);
            roleId = (await unitOfWork.Roles.GetAllAsync(TestContext.Current.CancellationToken)).First().Id;
            await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        }

        var account = await CreateAccountAsync("standard-mutations", []);
        using var initial = JsonDocument.Parse(
            await Client.GetStreamAsync($"/api/v1/serviceAccounts/{account.Id}", TestContext.Current.CancellationToken));
        var actorId = initial.RootElement.GetProperty("actorId").GetGuid();

        (await Client.PatchAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}",
            new { description = "Patched" },
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();
        (await Client.PostAsJsonAsync(
            "/api/v1/serviceAccounts/rename",
            new { id = account.Id, name = "standard-mutations-renamed" },
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();
        (await Client.PostAsJsonAsync(
            $"/api/v1/teams/{team.Id}/members",
            new { memberActorId = actorId },
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();
        (await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/roles",
            new { roleId },
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();

        var accessResponse = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}/resource-accesses",
            new
            {
                resourceType = "ServiceAccount",
                resourceId = account.Id,
                permissionLevel = "Read",
                specificPermissions = Array.Empty<string>(),
            },
            TestContext.Current.CancellationToken);
        accessResponse.EnsureSuccessStatusCode();
        using var accessJson = JsonDocument.Parse(
            await accessResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        var resourceAccessId = Assert.Single(
            accessJson.RootElement.GetProperty("resourceAccesses").EnumerateArray()).GetProperty("id").GetGuid();

        (await Client.DeleteAsync(
            $"/api/v1/serviceAccounts/{account.Id}/resource-accesses/{resourceAccessId}",
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();
        (await Client.DeleteAsync(
            $"/api/v1/serviceAccounts/{account.Id}/roles/{roleId}",
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();
        (await Client.DeleteAsync(
            $"/api/v1/teams/{team.Id}/members/{actorId}",
            TestContext.Current.CancellationToken)).EnsureSuccessStatusCode();

        var archiveRequest = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/serviceAccounts/")
        {
            Content = JsonContent.Create(new { ids = new[] { account.Id } }),
        };
        var archiveResponse = await Client.SendAsync(archiveRequest, TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NoContent, archiveResponse.StatusCode);

        await using var verificationScope = Services.CreateAsyncScope();
        var verification = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var archived = await verification.ServiceAccounts.GetAsync(account.Id, TestContext.Current.CancellationToken);
        Assert.NotNull(archived?.ArchivedAtUtc);
        Assert.Empty(await verification.ServiceAccounts.GetTeamIdsAsync(actorId, TestContext.Current.CancellationToken));
        Assert.DoesNotContain(roleId, await verification.Roles.GetActorRoleIdsAsync(actorId, TestContext.Current.CancellationToken));
        Assert.Empty(await verification.ResourceAccesses.GetAllByActorIdAsync(actorId, TestContext.Current.CancellationToken));
    }

    private async Task<(Guid Id, string Name)> CreateAccountAsync(string name, Guid[] teamIds)
    {
        var response = await Client.PostAsJsonAsync(
            "/api/v1/serviceAccounts/",
            new
            {
                name,
                isEnabled = true,
                teamIds,
                roleIds = Array.Empty<Guid>(),
                resourceAccesses = Array.Empty<object>(),
            },
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();
        using var json = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        return (json.RootElement.GetProperty("id").GetGuid(), json.RootElement.GetProperty("name").GetString()!);
    }

    private async Task<(Guid CredentialId, string Token, string? CacheControl)> CreateTokenAsync(
        Guid accountId,
        string name,
        DateTime? expiresAtUtc)
    {
        var response = await Client.PostAsJsonAsync(
            $"/api/v1/serviceAccounts/{accountId}/tokens",
            new { name, expiresAtUtc, neverExpires = !expiresAtUtc.HasValue },
            TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);
        using var json = JsonDocument.Parse(await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken));
        return (
            json.RootElement.GetProperty("id").GetGuid(),
            json.RootElement.GetProperty("token").GetString()!,
            response.Headers.CacheControl?.ToString());
    }

    private async Task UpdateEnabledAsync((Guid Id, string Name) account, bool isEnabled)
    {
        var response = await Client.PatchAsJsonAsync(
            $"/api/v1/serviceAccounts/{account.Id}",
            new
            {
                isEnabled,
            },
            TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();
    }
}
