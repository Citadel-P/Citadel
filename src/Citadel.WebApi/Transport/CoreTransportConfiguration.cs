using Microsoft.AspNetCore.HttpOverrides;
using Microsoft.AspNetCore.Server.Kestrel.Core;

namespace WebApi.Transport;

internal static class CoreTransportConfiguration
{
    public static void ConfigureServices(
        this WebApplicationBuilder builder,
        CoreTransportSettings settings)
    {
        builder.Services.AddSingleton(_ => settings);
        if (settings.Mode == CoreTransportMode.ReverseProxy)
        {
            builder.Services.Configure<ForwardedHeadersOptions>(options =>
            {
                options.ForwardedHeaders =
                    ForwardedHeaders.XForwardedFor
                    | ForwardedHeaders.XForwardedProto;
                options.ForwardLimit = settings.ForwardLimit;
                options.KnownIPNetworks.Clear();
                options.KnownProxies.Clear();
                foreach (var proxy in settings.KnownProxies)
                {
                    options.KnownProxies.Add(proxy);
                }

                foreach (var network in settings.KnownNetworks)
                {
                    options.KnownIPNetworks.Add(network);
                }
            });
        }

        builder.WebHost.ConfigureKestrel(options =>
        {
            if (settings.Mode == CoreTransportMode.Direct)
            {
                var certificate = settings.Certificate
                                  ?? throw new InvalidOperationException(
                                      "Direct transport mode has no server certificate.");
                options.ListenAnyIP(settings.ApiPort, listen =>
                {
                    listen.Protocols = HttpProtocols.Http1AndHttp2;
                    listen.UseHttps(https =>
                    {
                        https.ServerCertificate = certificate.Certificate;
                        if (certificate.Chain.Count > 0)
                        {
                            https.ServerCertificateChain = certificate.Chain;
                        }
                    });
                });
                options.ListenAnyIP(settings.EdgeGrpcPort, listen =>
                {
                    listen.Protocols = HttpProtocols.Http2;
                    listen.UseHttps(https =>
                    {
                        https.ServerCertificate = certificate.Certificate;
                        if (certificate.Chain.Count > 0)
                        {
                            https.ServerCertificateChain = certificate.Chain;
                        }
                    });
                });
                return;
            }

            options.ListenAnyIP(
                settings.ApiPort,
                listen => listen.Protocols = HttpProtocols.Http1);
            options.ListenAnyIP(
                settings.EdgeGrpcPort,
                listen => listen.Protocols = HttpProtocols.Http2);
        });
    }

    public static void ConfigurePipeline(
        this WebApplication app,
        CoreTransportSettings settings)
    {
        if (settings.Mode == CoreTransportMode.ReverseProxy)
        {
            app.UseForwardedHeaders();
        }

        if (settings.Mode == CoreTransportMode.Direct
            && !app.Environment.IsDevelopment())
        {
            app.UseHsts();
        }

        if (settings.Mode == CoreTransportMode.Disabled)
        {
            app.Logger.LogWarning(
                "Citadel transport security is disabled. API and Agent traffic is not encrypted.");
        }
        else
        {
            app.Logger.LogInformation(
                "Citadel transport mode {TransportMode}; public URL {PublicUrl}; Edge Agent gRPC URL {EdgeAgentGrpcUrl}",
                settings.Mode,
                settings.PublicUrl,
                settings.EdgeAgentGrpcUrl);
        }

        if (settings.Certificate is { } certificate)
        {
            app.Logger.LogInformation(
                "Loaded TLS certificate {CertificateSubject}; thumbprint {CertificateThumbprint}; expires {CertificateExpiresAt}",
                certificate.Certificate.Subject,
                certificate.Certificate.Thumbprint,
                certificate.Certificate.NotAfter.ToUniversalTime());
            if (certificate.ExpiresSoon)
            {
                app.Logger.LogWarning(
                    "The Citadel TLS certificate expires within 30 days at {CertificateExpiresAt}",
                    certificate.Certificate.NotAfter.ToUniversalTime());
            }
        }
    }
}
