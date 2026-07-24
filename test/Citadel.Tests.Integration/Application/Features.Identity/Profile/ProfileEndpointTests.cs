using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Profile;

public sealed class ProfileEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid SeededAdminUserId = Guid.Parse("10000000-0000-0000-0000-000000000001");

    [Fact]
    public async Task GetCurrentProfile_ReturnsAuthenticatedUser()
    {
        UseSession(await LoginAsync("CitadelTests/1.0 profile-read"));

        var response = await Client.GetAsync("/api/v1/profile", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await ReadJsonAsync(response);
        var root = json.RootElement;

        Assert.Equal(SeededAdminUserId, root.GetProperty("id").GetGuid());
        Assert.Equal("admin", root.GetProperty("displayName").GetString());
        Assert.Equal("admin@citadel.local", root.GetProperty("email").GetString());
        Assert.True(root.GetProperty("authentication").GetProperty("canChangePassword").GetBoolean());
        Assert.True(root.GetProperty("directRoles").GetArrayLength() > 0);
    }

    [Fact]
    public async Task UpdateCurrentProfile_UpdatesCurrentUserDisplayName()
    {
        UseSession(await LoginAsync("CitadelTests/1.0 profile-update"));

        var response = await Client.PatchAsync(
            "/api/v1/profile",
            JsonContent("""
            {
              "displayName": "updated-profile-name"
            }
            """),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var admin = await uow.Users.GetAsync(SeededAdminUserId, TestContext.Current.CancellationToken);

        Assert.Equal("updated-profile-name", admin?.Name);
    }

    [Fact]
    public async Task UpdateCurrentProfile_RejectsDuplicateDisplayName()
    {
        UseSession(await LoginAsync("CitadelTests/1.0 profile-update-duplicate"));
        await SeedUserAsync("duplicate-name", "duplicate-profile-name@citadel.local");

        var response = await Client.PatchAsync(
            "/api/v1/profile",
            JsonContent("""
            {
              "displayName": "duplicate-name"
            }
            """),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task GetPreferences_ReturnsDefaultsWithoutCreatingPreferencesRow()
    {
        UseSession(await LoginAsync("CitadelTests/1.0 preferences-read"));

        var response = await Client.GetAsync("/api/v1/profile/preferences", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await ReadJsonAsync(response);
        var root = json.RootElement;

        Assert.True(root.TryGetProperty("timeZone", out var timeZone), root.GetRawText());
        Assert.Equal(JsonValueKind.Null, timeZone.ValueKind);
        Assert.Equal("System", root.GetProperty("dateTimeFormat").GetString());
        Assert.Equal("System", root.GetProperty("theme").GetString());
        Assert.False(root.GetProperty("isPersisted").GetBoolean());

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var preferences = await uow.UserPreferences.GetAsync(SeededAdminUserId, TestContext.Current.CancellationToken);

        Assert.Null(preferences);
    }

    [Fact]
    public async Task PatchPreferences_SavesValidPreferences_AndRejectsInvalidTimezone()
    {
        UseSession(await LoginAsync("CitadelTests/1.0 preferences-patch"));

        var response = await Client.PatchAsync(
            "/api/v1/profile/preferences",
            MergePatchContent("""
            {
              "timeZone": "UTC",
              "dateTimeFormat": "TwentyFourHour",
              "theme": "Dark"
            }
            """),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        using (var json = await ReadJsonAsync(response))
        {
            var root = json.RootElement;
            Assert.Equal("UTC", root.GetProperty("timeZone").GetString());
            Assert.Equal("TwentyFourHour", root.GetProperty("dateTimeFormat").GetString());
            Assert.Equal("Dark", root.GetProperty("theme").GetString());
            Assert.True(root.GetProperty("isPersisted").GetBoolean());
        }

        var invalidResponse = await Client.PatchAsync(
            "/api/v1/profile/preferences",
            MergePatchContent("""
            {
              "timeZone": "Not/A_Timezone"
            }
            """),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, invalidResponse.StatusCode);
    }

    [Fact]
    public async Task ListSessions_ResolvesCurrentSessionFromRefreshCookie()
    {
        var login = await LoginAsync("CitadelTests/1.0 current-session");
        UseSession(login);

        var response = await Client.GetAsync("/api/v1/profile/sessions", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await ReadJsonAsync(response);
        var root = json.RootElement;
        var sessions = root.GetProperty("sessions");

        Assert.True(root.GetProperty("canRevokeOtherSessions").GetBoolean());
        Assert.Equal(1, sessions.GetArrayLength());
        Assert.True(sessions[0].GetProperty("isCurrent").GetBoolean());
        Assert.Equal("CitadelTests/1.0 current-session", sessions[0].GetProperty("userAgent").GetString());
    }

    [Fact]
    public async Task RevokeSession_CanDeleteOtherOwnedSession_ButNotCurrentSession()
    {
        _ = await LoginAsync("CitadelTests/1.0 old-session");
        var currentLogin = await LoginAsync("CitadelTests/1.0 current-session");
        UseSession(currentLogin);

        var sessions = await GetSessionsAsync();
        var currentSessionId = sessions.Single(x => x.IsCurrent).Id;
        var otherSessionId = sessions.Single(x => !x.IsCurrent).Id;

        var deleteOther = await Client.DeleteAsync($"/api/v1/profile/sessions/{otherSessionId}", TestContext.Current.CancellationToken);
        deleteOther.EnsureSuccessStatusCode();

        var deleteCurrent = await Client.DeleteAsync($"/api/v1/profile/sessions/{currentSessionId}", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.NotFound, deleteCurrent.StatusCode);

        var remaining = await GetSessionsAsync();
        Assert.Single(remaining);
        Assert.Equal(currentSessionId, remaining[0].Id);
    }

    [Fact]
    public async Task RevokeOtherSessions_RequiresCurrentSession_AndKeepsCurrentSession()
    {
        UseAccessTokenWithoutRefreshCookie(await LoginAsync("CitadelTests/1.0 no-current-session"));

        var missingCurrentResponse = await Client.DeleteAsync("/api/v1/profile/sessions", TestContext.Current.CancellationToken);
        Assert.Equal(HttpStatusCode.BadRequest, missingCurrentResponse.StatusCode);

        _ = await LoginAsync("CitadelTests/1.0 old-session");
        var currentLogin = await LoginAsync("CitadelTests/1.0 current-session");
        UseSession(currentLogin);

        var response = await Client.DeleteAsync("/api/v1/profile/sessions", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using (var json = await ReadJsonAsync(response))
        {
            Assert.Equal(2, json.RootElement.GetProperty("count").GetInt32());
        }

        var remaining = await GetSessionsAsync();
        Assert.Single(remaining);
        Assert.True(remaining[0].IsCurrent);
    }

    [Fact]
    public async Task ChangePassword_RejectsWrongPassword_AndRevokesOtherSessionsOnSuccess()
    {
        _ = await LoginAsync("CitadelTests/1.0 old-session");
        var currentLogin = await LoginAsync("CitadelTests/1.0 current-session");
        UseSession(currentLogin);

        var wrongPasswordResponse = await Client.PostAsync(
            "/api/v1/profile/change-password",
            JsonContent("""
            {
              "currentPassword": "wrong-password",
              "newPassword": "newPassword123"
            }
            """),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, wrongPasswordResponse.StatusCode);

        var response = await Client.PostAsync(
            "/api/v1/profile/change-password",
            JsonContent("""
            {
              "currentPassword": "admin123",
              "newPassword": "newPassword123"
            }
            """),
            TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        var remaining = await GetSessionsAsync();
        Assert.Single(remaining);
        Assert.True(remaining[0].IsCurrent);

        var newLogin = await LoginAsync("CitadelTests/1.0 new-password", "admin@citadel.local", "newPassword123");
        Assert.False(string.IsNullOrWhiteSpace(newLogin.AccessToken));
    }

    private async Task<LoginSession> LoginAsync(
        string userAgent,
        string emailOrName = "admin@citadel.local",
        string password = "admin123")
    {
        var request = new HttpRequestMessage(HttpMethod.Post, "/api/v1/authentication/login")
        {
            Content = JsonContent($$"""
            {
              "emailOrName": "{{emailOrName}}",
              "password": "{{password}}"
            }
            """)
        };
        request.Headers.UserAgent.ParseAdd(userAgent);

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await ReadJsonAsync(response);
        var accessToken = json.RootElement.GetProperty("accessToken").GetString()!;
        var refreshToken = GetRefreshTokenCookie(response);

        return new LoginSession(accessToken, refreshToken);
    }

    private void UseSession(LoginSession session)
    {
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", session.AccessToken);
        Client.DefaultRequestHeaders.Remove("Cookie");
        Client.DefaultRequestHeaders.Add("Cookie", $"{Constants.RefreshToken}={session.RefreshToken}");
    }

    private void UseAccessTokenWithoutRefreshCookie(LoginSession session)
    {
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", session.AccessToken);
        Client.DefaultRequestHeaders.Remove("Cookie");
    }

    private async Task<List<TestSession>> GetSessionsAsync()
    {
        var response = await Client.GetAsync("/api/v1/profile/sessions", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var json = await ReadJsonAsync(response);
        return json.RootElement.GetProperty("sessions")
            .EnumerateArray()
            .Select(x => new TestSession(
                x.GetProperty("id").GetGuid(),
                x.GetProperty("isCurrent").GetBoolean()))
            .ToList();
    }

    private async Task<(Guid UserId, Guid ActorId)> SeedUserAsync(string name, string email, string password = "password123")
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name));
        var user = new User(name, email, password, actor.Id, Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return (user.Id, actor.Id);
    }

    private static string GetRefreshTokenCookie(HttpResponseMessage response)
    {
        Assert.True(response.Headers.TryGetValues("Set-Cookie", out var setCookieHeaders));
        var prefix = $"{Constants.RefreshToken}=";

        foreach (var header in setCookieHeaders)
        {
            var cookie = header.Split(';', 2)[0].Trim();
            if (cookie.StartsWith(prefix, StringComparison.Ordinal) && cookie.Length > prefix.Length)
                return cookie[prefix.Length..];
        }

        throw new InvalidOperationException("Refresh token cookie was not set.");
    }

    private static StringContent JsonContent(string json) => new(json, Encoding.UTF8, "application/json");

    private static StringContent MergePatchContent(string json) => new(json, Encoding.UTF8, "application/merge-patch+json");

    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response)
        => await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

    private sealed record LoginSession(string AccessToken, string RefreshToken);

    private sealed record TestSession(Guid Id, bool IsCurrent);
}
