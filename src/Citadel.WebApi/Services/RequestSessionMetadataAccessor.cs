using Application.Services;
using Hosting.Common;
using Microsoft.AspNetCore.Http;

namespace WebApi.Services;

internal sealed class RequestSessionMetadataAccessor(IHttpContextAccessor httpContextAccessor) : IRequestSessionMetadataAccessor
{
    public RequestSessionMetadata GetCurrent()
    {
        var context = httpContextAccessor.HttpContext;
        return new RequestSessionMetadata(
            context?.Request.Headers.UserAgent.ToString(),
            context?.Connection.RemoteIpAddress?.ToString());
    }
}

internal sealed class RefreshTokenCookieService(IHttpContextAccessor httpContextAccessor) : IRefreshTokenCookieService
{
    public string? GetCurrent()
        => httpContextAccessor.HttpContext?.Request.Cookies[Constants.RefreshToken];

    public void Set(string refreshToken, DateTime expiresAt)
    {
        httpContextAccessor.HttpContext?.Response.Cookies.Append(
            Constants.RefreshToken,
            refreshToken,
            CookieOptionsFactory.Create(expiresAt));
    }

    public void Delete()
        => httpContextAccessor.HttpContext?.Response.Cookies.Delete(Constants.RefreshToken, CookieOptionsFactory.DeleteOptions());
}

internal static class CookieOptionsFactory
{
    private const string AuthenticationPath = "/api/v1/authentication";

    public static CookieOptions Create(DateTime expiresAt)
        => new()
        {
            HttpOnly = true,
            Secure = true,
            IsEssential = true,
            SameSite = SameSiteMode.None,
            Path = AuthenticationPath,
            Expires = new DateTimeOffset(DateTime.SpecifyKind(expiresAt, DateTimeKind.Utc)),
            MaxAge = expiresAt - DateTime.UtcNow
        };

    public static CookieOptions DeleteOptions()
        => new()
        {
            Secure = true,
            SameSite = SameSiteMode.None,
            Path = AuthenticationPath
        };
}

internal sealed class MfaChallengeCookieService(IHttpContextAccessor httpContextAccessor) : IMfaChallengeCookieService
{
    public Guid? GetCurrent()
    {
        var value = httpContextAccessor.HttpContext?.Request.Cookies[Constants.MfaChallenge];
        return Guid.TryParse(value, out var challengeId) ? challengeId : null;
    }

    public void Set(Guid challengeId, DateTime expiresAt)
        => Append(Constants.MfaChallenge, challengeId, expiresAt);

    public void Delete()
        => httpContextAccessor.HttpContext?.Response.Cookies.Delete(Constants.MfaChallenge, CookieOptionsFactory.DeleteOptions());

    private void Append(string name, Guid value, DateTime expiresAt)
    {
        httpContextAccessor.HttpContext?.Response.Cookies.Append(name, value.ToString(), CookieOptionsFactory.Create(expiresAt));
    }
}

internal sealed class MfaSetupCookieService(IHttpContextAccessor httpContextAccessor) : IMfaSetupCookieService
{
    public Guid? GetCurrent()
    {
        var value = httpContextAccessor.HttpContext?.Request.Cookies[Constants.MfaSetup];
        return Guid.TryParse(value, out var setupSessionId) ? setupSessionId : null;
    }

    public void Set(Guid setupSessionId, DateTime expiresAt)
    {
        httpContextAccessor.HttpContext?.Response.Cookies.Append(
            Constants.MfaSetup,
            setupSessionId.ToString(),
            CookieOptionsFactory.Create(expiresAt));
    }

    public void Delete()
        => httpContextAccessor.HttpContext?.Response.Cookies.Delete(Constants.MfaSetup, CookieOptionsFactory.DeleteOptions());
}
