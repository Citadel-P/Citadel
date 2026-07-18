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

public interface IMfaChallengeCookieService
{
    Guid? GetCurrent();
    void Set(Guid challengeId, DateTime expiresAt);
    void Delete();
}

public interface IMfaSetupCookieService
{
    Guid? GetCurrent();
    void Set(Guid setupSessionId, DateTime expiresAt);
    void Delete();
}
