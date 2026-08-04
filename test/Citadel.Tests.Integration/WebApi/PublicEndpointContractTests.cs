using System.Net.Http.Headers;
using System.Text;
using System.Text.RegularExpressions;
using Microsoft.AspNetCore.Http.Metadata;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.Routing;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.WebApi;

public sealed partial class PublicEndpointContractTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid MissingId = Guid.Parse("ffffffff-ffff-ffff-ffff-ffffffffffff");

    [Fact]
    public async Task EveryApiEndpoint_ShouldResolveAndAvoidServerErrorsForPlaceholderInput()
    {
        using var client = Factory.CreateClient(new WebApplicationFactoryClientOptions
        {
            AllowAutoRedirect = false
        });
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());

        var dataSource = Services.GetRequiredService<EndpointDataSource>();
        var endpoints = dataSource.Endpoints
            .OfType<RouteEndpoint>()
            .SelectMany(endpoint =>
            {
                var methods = endpoint.Metadata.GetMetadata<IHttpMethodMetadata>()?.HttpMethods ?? [];
                return methods.Select(method => new EndpointCase(
                    method,
                    endpoint.RoutePattern.RawText ?? string.Empty,
                    endpoint.Metadata.GetMetadata<IEndpointNameMetadata>()?.EndpointName ?? endpoint.DisplayName ?? "unnamed"));
            })
            .Where(endpoint => IsPublicEndpoint(endpoint.Route))
            .OrderBy(endpoint => endpoint.Route, StringComparer.Ordinal)
            .ThenBy(endpoint => endpoint.Method, StringComparer.Ordinal)
            .ToArray();

        Assert.NotEmpty(endpoints);
        var failures = new List<string>();
        foreach (var endpoint in endpoints)
        {
            using var request = CreateRequest(endpoint);
            try
            {
                using var response = await client.SendAsync(
                    request,
                    HttpCompletionOption.ResponseHeadersRead,
                    TestContext.Current.CancellationToken);
                if ((int)response.StatusCode >= 500)
                {
                    failures.Add(
                        $"{endpoint.Method} {endpoint.Route} ({endpoint.Name}) returned {(int)response.StatusCode} {response.StatusCode}.");
                }
            }
            catch (Exception exception)
            {
                failures.Add($"{endpoint.Method} {endpoint.Route} ({endpoint.Name}) threw {exception.GetType().Name}: {exception.Message}");
            }
        }

        Assert.True(
            failures.Count == 0,
            $"The following endpoints returned server errors:{Environment.NewLine}{string.Join(Environment.NewLine, failures)}");
    }

    [Theory]
    [InlineData("PATCH", "/api/v1/gitAccounts/ffffffff-ffff-ffff-ffff-ffffffffffff")]
    [InlineData("POST", "/api/v1/gitRepositories/")]
    [InlineData("PATCH", "/api/v1/gitRepositories/ffffffff-ffff-ffff-ffff-ffffffffffff")]
    [InlineData("POST", "/api/v1/gitRepositories/ffffffff-ffff-ffff-ffff-ffffffffffff/sync")]
    [InlineData("DELETE", "/api/v1/platforms/")]
    [InlineData("POST", "/api/v1/platforms/")]
    [InlineData("PATCH", "/api/v1/platforms/ffffffff-ffff-ffff-ffff-ffffffffffff")]
    public async Task MalformedInput_ShouldNotReturnServerError(string method, string path)
    {
        using var client = Factory.CreateClient(new WebApplicationFactoryClientOptions
        {
            AllowAutoRedirect = false
        });
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", CreateJwtToken());
        using var request = new HttpRequestMessage(new HttpMethod(method), path)
        {
            Content = new StringContent("{}", Encoding.UTF8, "application/json")
        };

        using var response = await client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.True(
            (int)response.StatusCode < 500,
            $"{method} {path} returned {(int)response.StatusCode} {response.StatusCode}.");
    }

    private static HttpRequestMessage CreateRequest(EndpointCase endpoint)
    {
        var route = RouteParameterRegex().Replace(
            endpoint.Route.TrimStart('/'),
            static match => Placeholder(match.Groups[1].Value));
        var request = new HttpRequestMessage(new HttpMethod(endpoint.Method), $"/{route}");
        if (endpoint.Method is not "GET" and not "HEAD")
            request.Content = new StringContent("{}", Encoding.UTF8, "application/json");
        return request;
    }

    private static bool IsPublicEndpoint(string route)
    {
        var normalizedRoute = route.TrimStart('/');
        return normalizedRoute.StartsWith("api/v1", StringComparison.OrdinalIgnoreCase)
               || normalizedRoute.StartsWith("listener/", StringComparison.OrdinalIgnoreCase);
    }

    private static string Placeholder(string routeParameter)
    {
        if (routeParameter.Contains("guid", StringComparison.OrdinalIgnoreCase)
            || routeParameter.Contains("id", StringComparison.OrdinalIgnoreCase))
            return MissingId.ToString("D");
        if (routeParameter.Contains("int", StringComparison.OrdinalIgnoreCase)
            || routeParameter.Contains("long", StringComparison.OrdinalIgnoreCase))
            return "1";
        return "missing";
    }

    [GeneratedRegex("\\{([^{}]+)\\}", RegexOptions.CultureInvariant)]
    private static partial Regex RouteParameterRegex();

    private sealed record EndpointCase(string Method, string Route, string Name);
}
