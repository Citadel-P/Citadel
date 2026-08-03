using Hosting.DockerClient.Services;

namespace Tests.Unit.DockerClient;

public sealed class DockerApiVersionTests
{
    [Theory]
    [InlineData("1.41", "1.41")]
    [InlineData("1.45", "1.45")]
    [InlineData("1.49", "1.49")]
    [InlineData("1.53", "1.49")]
    public void TryNegotiate_ShouldUseHighestMutuallySupportedVersion(string daemonMaximum, string expected)
    {
        Assert.True(DockerApiVersion.TryNegotiate(daemonMaximum, out var negotiated));
        Assert.Equal(expected, negotiated);
    }

    [Theory]
    [InlineData("1.49", "1.41", "1.49")]
    [InlineData("1.53", "1.49", "1.49")]
    public void TryNegotiate_ShouldHonorDaemonMinimum(
        string daemonMaximum,
        string daemonMinimum,
        string expected)
    {
        Assert.True(DockerApiVersion.TryNegotiate(daemonMaximum, daemonMinimum, out var negotiated));
        Assert.Equal(expected, negotiated);
    }

    [Theory]
    [InlineData("1.53", "1.50")]
    [InlineData("1.49", "invalid")]
    public void TryNegotiate_ShouldRejectIncompatibleDaemonMinimum(
        string daemonMaximum,
        string daemonMinimum)
    {
        Assert.False(DockerApiVersion.TryNegotiate(daemonMaximum, daemonMinimum, out var negotiated));
        Assert.Empty(negotiated);
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("invalid")]
    [InlineData("1.40")]
    public void TryNegotiate_ShouldRejectUnsupportedVersion(string? daemonMaximum)
    {
        Assert.False(DockerApiVersion.TryNegotiate(daemonMaximum, out var negotiated));
        Assert.Empty(negotiated);
    }

    [Fact]
    public async Task Handler_ShouldNegotiateOnceAndPrefixDockerRequests()
    {
        var requests = new List<Uri>();
        using var state = new DockerApiVersionState();
        using var handler = new DockerApiVersionHandler(state)
        {
            InnerHandler = new StubHandler(request =>
            {
                requests.Add(request.RequestUri!);
                return request.RequestUri!.AbsolutePath == "/version"
                    ? JsonResponse("""{"ApiVersion":"1.53","MinAPIVersion":"1.41"}""")
                    : new HttpResponseMessage(System.Net.HttpStatusCode.OK);
            })
        };
        using var invoker = new HttpMessageInvoker(handler);

        using var first = await invoker.SendAsync(
            new HttpRequestMessage(HttpMethod.Get, "http://localhost/info"),
            TestContext.Current.CancellationToken);
        using var second = await invoker.SendAsync(
            new HttpRequestMessage(HttpMethod.Get, "http://localhost/containers/json?all=true"),
            TestContext.Current.CancellationToken);

        Assert.Equal(
            ["/version", "/v1.49/info", "/v1.49/containers/json"],
            requests.Select(static request => request.AbsolutePath));
        Assert.Equal("?all=true", requests[2].Query);
        Assert.Equal("1.49", state.Current?.NegotiatedVersion);
        Assert.Equal("1.41", state.Current?.DaemonMinimumVersion);
    }

    [Fact]
    public async Task Handler_ShouldRejectDaemonWhoseMinimumIsTooNew()
    {
        using var state = new DockerApiVersionState();
        using var handler = new DockerApiVersionHandler(state)
        {
            InnerHandler = new StubHandler(_ =>
                JsonResponse("""{"ApiVersion":"1.53","MinAPIVersion":"1.50"}"""))
        };
        using var invoker = new HttpMessageInvoker(handler);

        var exception = await Assert.ThrowsAsync<HttpRequestException>(() => invoker.SendAsync(
            new HttpRequestMessage(HttpMethod.Get, "http://localhost/info"),
            TestContext.Current.CancellationToken));

        Assert.Contains("not compatible", exception.Message, StringComparison.OrdinalIgnoreCase);
        Assert.Null(state.Current);
    }

    [Fact]
    public async Task Handler_ShouldRenegotiateAfterDaemonRejectsCachedVersion()
    {
        var requests = new List<string>();
        var versionRequestCount = 0;
        using var state = new DockerApiVersionState();
        using var handler = new DockerApiVersionHandler(state)
        {
            InnerHandler = new StubHandler(request =>
            {
                requests.Add(request.RequestUri!.AbsolutePath);
                if (request.RequestUri.AbsolutePath == "/version")
                {
                    versionRequestCount++;
                    return versionRequestCount == 1
                        ? JsonResponse("""{"ApiVersion":"1.49","MinAPIVersion":"1.41"}""")
                        : JsonResponse("""{"ApiVersion":"1.45","MinAPIVersion":"1.41"}""");
                }

                return request.RequestUri.AbsolutePath == "/v1.49/info"
                    ? new HttpResponseMessage(System.Net.HttpStatusCode.BadRequest)
                    : new HttpResponseMessage(System.Net.HttpStatusCode.OK);
            })
        };
        using var invoker = new HttpMessageInvoker(handler);

        using var rejected = await invoker.SendAsync(
            new HttpRequestMessage(HttpMethod.Get, "http://localhost/info"),
            TestContext.Current.CancellationToken);
        using var recovered = await invoker.SendAsync(
            new HttpRequestMessage(HttpMethod.Get, "http://localhost/containers/json"),
            TestContext.Current.CancellationToken);

        Assert.Equal(System.Net.HttpStatusCode.BadRequest, rejected.StatusCode);
        Assert.Equal(System.Net.HttpStatusCode.OK, recovered.StatusCode);
        Assert.Equal(
            ["/version", "/v1.49/info", "/version", "/v1.45/containers/json"],
            requests);
        Assert.Equal("1.45", state.Current?.NegotiatedVersion);
    }

    private static HttpResponseMessage JsonResponse(string json)
        => new(System.Net.HttpStatusCode.OK)
        {
            Content = new StringContent(json, System.Text.Encoding.UTF8, "application/json")
        };

    private sealed class StubHandler(Func<HttpRequestMessage, HttpResponseMessage> send) : HttpMessageHandler
    {
        protected override Task<HttpResponseMessage> SendAsync(
            HttpRequestMessage request,
            CancellationToken cancellationToken)
            => Task.FromResult(send(request));
    }
}
