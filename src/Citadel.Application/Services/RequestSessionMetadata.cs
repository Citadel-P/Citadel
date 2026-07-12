namespace Application.Services;

public interface IRequestSessionMetadataAccessor
{
    RequestSessionMetadata GetCurrent();
}

public sealed record RequestSessionMetadata(string? UserAgent, string? IpAddress);

public interface IRefreshTokenCookieService
{
    string? GetCurrent();
    void Set(string refreshToken, DateTime expiresAt);
    void Delete();
}
