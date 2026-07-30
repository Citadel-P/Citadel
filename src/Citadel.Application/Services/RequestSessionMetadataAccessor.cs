using Hosting.Common;
using Microsoft.AspNetCore.Http;

namespace Application.Services;

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

internal sealed class RefreshTokenCookieService(
    IHttpContextAccessor httpContextAccessor,
    AuthenticationCookiePolicy cookiePolicy) : IRefreshTokenCookieService
{
    public string? GetCurrent()
        => AuthenticationCookieHelper.GetRequestCookieValue(
            httpContextAccessor.HttpContext?.Request,
            Constants.RefreshToken);

    public void Set(string refreshToken, DateTime expiresAt)
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.RefreshToken, cookiePolicy.Secure);
        context.Response.Cookies.Append(
            Constants.RefreshToken,
            refreshToken,
            AuthenticationCookieHelper.CreateOptions(expiresAt, cookiePolicy.Secure));
    }

    public void Delete()
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.RefreshToken, cookiePolicy.Secure);
    }
}

internal static class AuthenticationCookieHelper
{
    private const string AuthenticationPath = "/api/v1/authentication";
    private static readonly string[] KnownPaths = [AuthenticationPath, "/"];

    public static CookieOptions CreateOptions(DateTime expiresAt, bool secure)
    {
        return new CookieOptions
        {
            HttpOnly = true,
            Secure = secure,
            IsEssential = true,
            SameSite = secure ? SameSiteMode.None : SameSiteMode.Lax,
            Path = AuthenticationPath,
            Expires = new DateTimeOffset(DateTime.SpecifyKind(expiresAt, DateTimeKind.Utc)),
            MaxAge = expiresAt - DateTime.UtcNow
        };
    }

    public static void Delete(HttpResponse response, string cookieName, bool secure)
    {
        foreach (var path in KnownPaths)
            response.Cookies.Delete(cookieName, DeleteOptions(secure, path));
    }

    public static string? GetRequestCookieValue(HttpRequest? request, string cookieName)
    {
        if (request is null)
            return null;

        foreach (var header in request.Headers.Cookie)
        {
            foreach (var part in header?.Split(';', StringSplitOptions.RemoveEmptyEntries) ?? [])
            {
                var separatorIndex = part.IndexOf('=');
                if (separatorIndex <= 0)
                    continue;

                var name = part[..separatorIndex].Trim();
                if (!string.Equals(name, cookieName, StringComparison.Ordinal))
                    continue;

                var value = part[(separatorIndex + 1)..].Trim();
                if (!string.IsNullOrWhiteSpace(value))
                    return Uri.UnescapeDataString(value);
            }
        }

        return request.Cookies.TryGetValue(cookieName, out var parsedValue)
            ? parsedValue
            : null;
    }

    private static CookieOptions DeleteOptions(bool secure, string path)
        => new()
        {
            Secure = secure,
            SameSite = secure ? SameSiteMode.None : SameSiteMode.Lax,
            Path = path
        };
}

internal sealed class MfaChallengeCookieService(
    IHttpContextAccessor httpContextAccessor,
    AuthenticationCookiePolicy cookiePolicy) : IMfaChallengeCookieService
{
    public Guid? GetCurrent()
    {
        var value = httpContextAccessor.HttpContext?.Request.Cookies[Constants.MfaChallenge];
        return Guid.TryParse(value, out var challengeId) ? challengeId : null;
    }

    public void Set(Guid challengeId, DateTime expiresAt)
        => Append(Constants.MfaChallenge, challengeId, expiresAt);

    public void Delete()
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaChallenge, cookiePolicy.Secure);
    }

    private void Append(string name, Guid value, DateTime expiresAt)
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, name, cookiePolicy.Secure);
        context.Response.Cookies.Append(
            name,
            value.ToString(),
            AuthenticationCookieHelper.CreateOptions(expiresAt, cookiePolicy.Secure));
    }
}

internal sealed class MfaSetupCookieService(
    IHttpContextAccessor httpContextAccessor,
    AuthenticationCookiePolicy cookiePolicy) : IMfaSetupCookieService
{
    public Guid? GetCurrent()
    {
        var value = httpContextAccessor.HttpContext?.Request.Cookies[Constants.MfaSetup];
        return Guid.TryParse(value, out var setupSessionId) ? setupSessionId : null;
    }

    public void Set(Guid setupSessionId, DateTime expiresAt)
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaSetup, cookiePolicy.Secure);
        context.Response.Cookies.Append(
            Constants.MfaSetup,
            setupSessionId.ToString(),
            AuthenticationCookieHelper.CreateOptions(expiresAt, cookiePolicy.Secure));
    }

    public void Delete()
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaSetup, cookiePolicy.Secure);
    }
}
