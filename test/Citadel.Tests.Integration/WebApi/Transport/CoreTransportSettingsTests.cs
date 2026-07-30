using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Configuration;
using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using WebApi.Transport;

namespace Tests.Integration.WebApi.Transport;

public sealed class CoreTransportSettingsTests
{
    [Fact]
    public void Load_RejectsInvalidMode()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "invalid"));

        var exception = Assert.Throws<InvalidOperationException>(
            () => CoreTransportSettings.Load(configuration));

        Assert.Contains("Transport:Mode", exception.Message);
    }

    [Fact]
    public void Load_AcceptsExplicitDisabledMode()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "Disabled"));

        using var settings = CoreTransportSettings.Load(configuration);

        Assert.Equal(CoreTransportMode.Disabled, settings.Mode);
        Assert.Equal(
            "http://localhost:8000/",
            settings.PublicUrl.AbsoluteUri);
        Assert.Equal(
            "http://localhost:8001/",
            settings.EdgeAgentGrpcUrl.AbsoluteUri);
        Assert.False(settings.SecureCookies);
    }

    [Fact]
    public void Load_ConfiguresRestrictedReverseProxy()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "reverseproxy"),
            ("Transport:PublicUrl", "https://citadel.example.com"),
            (
                "EdgeAgent:PublicGrpcUrl",
                "https://edge.citadel.example.com:8444"),
            (
                "Transport:ForwardedHeaders:KnownProxies",
                "10.0.0.10,10.0.0.11"),
            (
                "Transport:ForwardedHeaders:KnownNetworks",
                "172.30.0.0/24"),
            ("AllowedHosts", "citadel.example.com;edge.citadel.example.com"));

        using var settings = CoreTransportSettings.Load(configuration);

        Assert.Equal(CoreTransportMode.ReverseProxy, settings.Mode);
        Assert.Equal(2, settings.KnownProxies.Count);
        Assert.Single(settings.KnownNetworks);
        Assert.True(settings.SecureCookies);
    }

    [Fact]
    public void Load_RejectsReverseProxyWithoutTrustedUpstream()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "ReverseProxy"),
            ("Transport:PublicUrl", "https://citadel.example.com"),
            (
                "EdgeAgent:PublicGrpcUrl",
                "https://citadel.example.com:8444"),
            ("AllowedHosts", "citadel.example.com"));

        var exception = Assert.Throws<InvalidOperationException>(
            () => CoreTransportSettings.Load(configuration));

        Assert.Contains("KnownProxies or KnownNetworks", exception.Message);
    }

    [Fact]
    public void Load_RejectsNumericTransportMode()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "2"));

        var exception = Assert.Throws<InvalidOperationException>(
            () => CoreTransportSettings.Load(configuration));

        Assert.Contains("ReverseProxy, Direct, or Disabled", exception.Message);
    }

    [Fact]
    public void Load_RejectsWildcardAllowedHostsInProduction()
    {
        var configuration = BuildConfiguration(
            ("Transport:Mode", "ReverseProxy"),
            ("Transport:PublicUrl", "https://citadel.example.com"),
            (
                "EdgeAgent:PublicGrpcUrl",
                "https://citadel.example.com:8444"),
            (
                "Transport:ForwardedHeaders:KnownProxies",
                "10.0.0.10"),
            ("AllowedHosts", "*"));

        var exception = Assert.Throws<InvalidOperationException>(
            () => CoreTransportSettings.Load(configuration));

        Assert.Contains("AllowedHosts", exception.Message);
    }

    [Fact]
    public void Load_AcceptsDirectPemCertificateWithMatchingSan()
    {
        var directory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-core-tls-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            var (certificatePath, keyPath) = WritePemCertificate(
                directory,
                "citadel.example.com");
            var configuration = BuildConfiguration(
                ("Transport:Mode", "Direct"),
                (
                    "Transport:PublicUrl",
                    "https://citadel.example.com:8000"),
                (
                    "EdgeAgent:PublicGrpcUrl",
                    "https://citadel.example.com:8001"),
                ("AllowedHosts", "citadel.example.com"),
                ("Transport:Certificate:Path", certificatePath),
                (
                    "Transport:Certificate:PrivateKeyPath",
                    keyPath));

            using var settings = CoreTransportSettings.Load(configuration);

            Assert.Equal(CoreTransportMode.Direct, settings.Mode);
            Assert.NotNull(settings.Certificate);
        }
        finally
        {
            Directory.Delete(directory, recursive: true);
        }
    }

    [Fact]
    public void Load_RejectsDirectCertificateWithWrongSan()
    {
        var directory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-core-tls-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            var (certificatePath, keyPath) = WritePemCertificate(
                directory,
                "other.example.com");
            var configuration = BuildConfiguration(
                ("Transport:Mode", "Direct"),
                (
                    "Transport:PublicUrl",
                    "https://citadel.example.com:8000"),
                (
                    "EdgeAgent:PublicGrpcUrl",
                    "https://citadel.example.com:8001"),
                ("AllowedHosts", "citadel.example.com"),
                ("Transport:Certificate:Path", certificatePath),
                (
                    "Transport:Certificate:PrivateKeyPath",
                    keyPath));

            var exception = Assert.Throws<InvalidOperationException>(
                () => CoreTransportSettings.Load(configuration));

            Assert.Contains("subject alternative name", exception.Message);
        }
        finally
        {
            Directory.Delete(directory, recursive: true);
        }
    }

    [Fact]
    public async Task DirectMode_ServesApiAndEdgeListenersOverTls()
    {
        var directory = Path.Combine(
            Path.GetTempPath(),
            $"citadel-core-tls-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            var (apiPort, edgePort) = GetAvailablePorts();
            var (certificatePath, keyPath) = WritePemCertificate(
                directory,
                "localhost");
            var configuration = BuildConfiguration(
                ("Transport:Mode", "Direct"),
                (
                    "Transport:PublicUrl",
                    $"https://localhost:{apiPort}"),
                (
                    "Transport:ApiPort",
                    apiPort.ToString()),
                (
                    "Transport:EdgeGrpcPort",
                    edgePort.ToString()),
                (
                    "EdgeAgent:PublicGrpcUrl",
                    $"https://localhost:{edgePort}"),
                ("AllowedHosts", "localhost"),
                ("Transport:Certificate:Path", certificatePath),
                (
                    "Transport:Certificate:PrivateKeyPath",
                    keyPath));
            using var settings = CoreTransportSettings.Load(configuration);
            var builder = WebApplication.CreateSlimBuilder();
            builder.ConfigureServices(settings);
            await using var app = builder.Build();
            app.MapGet("/health", () => Results.Ok());
            await app.StartAsync(TestContext.Current.CancellationToken);

            using var handler = new HttpClientHandler
            {
                ServerCertificateCustomValidationCallback =
                    (_, certificate, _, _) =>
                        certificate?.GetCertHashString()
                        == settings.Certificate!.Certificate
                            .GetCertHashString()
            };
            using var client = new HttpClient(handler);

            using var apiResponse = await client.GetAsync(
                $"https://localhost:{apiPort}/health",
                TestContext.Current.CancellationToken);
            using var edgeRequest = new HttpRequestMessage(
                HttpMethod.Get,
                $"https://localhost:{edgePort}/health")
            {
                Version = HttpVersion.Version20,
                VersionPolicy = HttpVersionPolicy.RequestVersionExact
            };
            using var edgeResponse = await client.SendAsync(
                edgeRequest,
                TestContext.Current.CancellationToken);

            Assert.Equal(HttpStatusCode.OK, apiResponse.StatusCode);
            Assert.Equal(HttpStatusCode.OK, edgeResponse.StatusCode);
            Assert.Equal(HttpVersion.Version20, edgeResponse.Version);
        }
        finally
        {
            Directory.Delete(directory, recursive: true);
        }
    }

    [Fact]
    public async Task ReverseProxyMode_IgnoresForwardedHeadersFromUntrustedClient()
    {
        var (apiPort, edgePort) = GetAvailablePorts();
        var configuration = BuildConfiguration(
            ("Transport:Mode", "ReverseProxy"),
            ("Transport:PublicUrl", $"https://localhost:{apiPort}"),
            ("Transport:ApiPort", apiPort.ToString()),
            ("Transport:EdgeGrpcPort", edgePort.ToString()),
            (
                "EdgeAgent:PublicGrpcUrl",
                $"https://localhost:{edgePort}"),
            (
                "Transport:ForwardedHeaders:KnownProxies",
                "192.0.2.1"),
            ("AllowedHosts", "localhost"));
        using var settings = CoreTransportSettings.Load(configuration);
        var builder = WebApplication.CreateSlimBuilder();
        builder.ConfigureServices(settings);
        await using var app = builder.Build();
        app.ConfigurePipeline(settings);
        app.MapGet(
            "/forwarded-header-probe",
            (HttpContext context) => Results.Text(
                $"{context.Request.Scheme}|{context.Connection.RemoteIpAddress}"));
        await app.StartAsync(TestContext.Current.CancellationToken);

        using var client = new HttpClient();
        using var request = new HttpRequestMessage(
            HttpMethod.Get,
            $"http://localhost:{apiPort}/forwarded-header-probe");
        request.Headers.TryAddWithoutValidation(
            "X-Forwarded-Proto",
            "https");
        request.Headers.TryAddWithoutValidation(
            "X-Forwarded-For",
            "203.0.113.9");

        using var response = await client.SendAsync(
            request,
            TestContext.Current.CancellationToken);
        var body = await response.Content.ReadAsStringAsync(
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
        Assert.StartsWith("http|", body);
        Assert.DoesNotContain("203.0.113.9", body);
    }

    private static IConfiguration BuildConfiguration(
        params (string Key, string Value)[] values)
        => new ConfigurationBuilder()
            .AddInMemoryCollection(
                values.ToDictionary(
                    item => item.Key,
                    item => (string?)item.Value))
            .Build();

    private static (string CertificatePath, string KeyPath)
        WritePemCertificate(string directory, string dnsName)
    {
        using var key = RSA.Create(2048);
        var request = new CertificateRequest(
            $"CN={dnsName}",
            key,
            HashAlgorithmName.SHA256,
            RSASignaturePadding.Pkcs1);
        var subjectAlternativeNames = new SubjectAlternativeNameBuilder();
        subjectAlternativeNames.AddDnsName(dnsName);
        request.CertificateExtensions.Add(
            subjectAlternativeNames.Build());
        request.CertificateExtensions.Add(
            new X509EnhancedKeyUsageExtension(
                new OidCollection
                {
                    new("1.3.6.1.5.5.7.3.1")
                },
                critical: false));
        using var certificate = request.CreateSelfSigned(
            DateTimeOffset.UtcNow.AddMinutes(-1),
            DateTimeOffset.UtcNow.AddDays(7));

        var certificatePath = Path.Combine(directory, "core.crt");
        var keyPath = Path.Combine(directory, "core.key");
        File.WriteAllText(
            certificatePath,
            certificate.ExportCertificatePem());
        File.WriteAllText(keyPath, key.ExportPkcs8PrivateKeyPem());
        return (certificatePath, keyPath);
    }

    private static (int ApiPort, int EdgePort) GetAvailablePorts()
    {
        var apiListener = new TcpListener(IPAddress.Loopback, 0);
        var edgeListener = new TcpListener(IPAddress.Loopback, 0);
        apiListener.Start();
        edgeListener.Start();
        try
        {
            return (
                ((IPEndPoint)apiListener.LocalEndpoint).Port,
                ((IPEndPoint)edgeListener.LocalEndpoint).Port);
        }
        finally
        {
            apiListener.Stop();
            edgeListener.Stop();
        }
    }
}
