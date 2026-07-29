using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Text.Json;

namespace Tests.Acceptance.Infrastructure;

internal static class InitialAdministratorSession
{
    private const string Name = "admin";
    private const string Email = "admin@citadel.local";
    private const string Password = "citadel-acceptance-admin-password";

    public static async Task AuthenticateAsync(
        HttpClient client,
        string target,
        CancellationToken cancellationToken)
    {
        using var statusResponse = await client.GetAsync(
            "/api/v1/setup/status",
            cancellationToken);
        var statusBody = await statusResponse.Content.ReadAsStringAsync(cancellationToken);
        Assert.True(
            statusResponse.IsSuccessStatusCode,
            $"{target} setup status failed with HTTP {(int)statusResponse.StatusCode}: {statusBody}");

        using var statusJson = JsonDocument.Parse(statusBody);
        var requiresSetup = statusJson.RootElement
            .GetProperty("requiresSetup")
            .GetBoolean();

        using var authenticationResponse = requiresSetup
            ? await client.PostAsJsonAsync(
                "/api/v1/setup/initialize",
                new
                {
                    name = Name,
                    email = Email,
                    password = Password
                },
                cancellationToken)
            : await client.PostAsJsonAsync(
                "/api/v1/authentication/login",
                new
                {
                    emailOrName = Email,
                    password = Password
                },
                cancellationToken);

        var authenticationBody = await authenticationResponse.Content
            .ReadAsStringAsync(cancellationToken);
        Assert.True(
            authenticationResponse.IsSuccessStatusCode,
            $"""
            {target} administrator {(requiresSetup ? "initialization" : "login")} failed with HTTP {(int)authenticationResponse.StatusCode}.
            {authenticationBody}
            """);

        using var authenticationJson = JsonDocument.Parse(authenticationBody);
        var accessToken = authenticationJson.RootElement
            .GetProperty("accessToken")
            .GetString();
        Assert.False(string.IsNullOrWhiteSpace(accessToken));
        client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", accessToken);
    }
}
