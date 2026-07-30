namespace WebApi.Transport;

public interface ICitadelPublicEndpoints
{
    Uri PublicUrl { get; }
    Uri EdgeAgentGrpcUrl { get; }
    string BuildOidcCallback(Guid providerId);
    string BuildApplicationRoot();
}

internal sealed class CitadelPublicEndpoints(CoreTransportSettings settings)
    : ICitadelPublicEndpoints
{
    public Uri PublicUrl => settings.PublicUrl;
    public Uri EdgeAgentGrpcUrl => settings.EdgeAgentGrpcUrl;

    public string BuildOidcCallback(Guid providerId)
        => new Uri(
            PublicUrl,
            $"/api/v1/authentication/oidc/{providerId:D}/callback").AbsoluteUri;

    public string BuildApplicationRoot() => PublicUrl.AbsoluteUri;
}
