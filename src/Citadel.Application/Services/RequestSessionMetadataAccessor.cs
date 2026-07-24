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

internal sealed class RefreshTokenCookieService(IHttpContextAccessor httpContextAccessor) : IRefreshTokenCookieService
{
    public string? GetCurrent()
        => AuthenticationCookieHelper.GetRequestCookieValue(
            httpContextAccessor.HttpContext?.Request,
            Constants.RefreshToken);

    public void Set(string refreshToken, DateTime expiresAt)
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.RefreshToken, context.Request);
        context.Response.Cookies.Append(
            Constants.RefreshToken,
            refreshToken,
            AuthenticationCookieHelper.CreateOptions(expiresAt, context.Request));
    }

    public void Delete()
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.RefreshToken, context.Request);
    }
}

internal static class AuthenticationCookieHelper
{
    private const string AuthenticationPath = "/api/v1/authentication";
    private static readonly string[] KnownPaths = [AuthenticationPath, "/"];

    public static CookieOptions CreateOptions(DateTime expiresAt, HttpRequest request)
    {
        var isSecure = IsSecureRequest(request);

        return new CookieOptions
        {
            HttpOnly = true,
            Secure = isSecure,
            IsEssential = true,
            SameSite = isSecure ? SameSiteMode.None : SameSiteMode.Lax,
            Path = AuthenticationPath,
            Expires = new DateTimeOffset(DateTime.SpecifyKind(expiresAt, DateTimeKind.Utc)),
            MaxAge = expiresAt - DateTime.UtcNow
        };
    }

    public static void Delete(HttpResponse response, string cookieName, HttpRequest request)
    {
        foreach (var path in KnownPaths)
            response.Cookies.Delete(cookieName, DeleteOptions(request, path));
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

    private static CookieOptions DeleteOptions(HttpRequest request, string path)
    {
        var isSecure = IsSecureRequest(request);

        return new CookieOptions
        {
            Secure = isSecure,
            SameSite = isSecure ? SameSiteMode.None : SameSiteMode.Lax,
            Path = path
        };
    }

    private static bool IsSecureRequest(HttpRequest request)
        => request.IsHttps;
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
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaChallenge, context.Request);
    }

    private void Append(string name, Guid value, DateTime expiresAt)
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, name, context.Request);
        context.Response.Cookies.Append(name, value.ToString(), AuthenticationCookieHelper.CreateOptions(expiresAt, context.Request));
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
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaSetup, context.Request);
        context.Response.Cookies.Append(
            Constants.MfaSetup,
            setupSessionId.ToString(),
            AuthenticationCookieHelper.CreateOptions(expiresAt, context.Request));
    }

    public void Delete()
    {
        var context = httpContextAccessor.HttpContext;
        if (context is null) return;

        AuthenticationCookieHelper.Delete(context.Response, Constants.MfaSetup, context.Request);
    }
}
