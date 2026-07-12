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
        httpContextAccessor.HttpContext?.Response.Cookies.Append(Constants.RefreshToken, refreshToken, new CookieOptions
        {
            HttpOnly = true,
            Secure = true,
            IsEssential = true,
            SameSite = SameSiteMode.Strict,
            Expires = new DateTimeOffset(DateTime.SpecifyKind(expiresAt, DateTimeKind.Utc)),
            MaxAge = expiresAt - DateTime.UtcNow
        });
    }

    public void Delete()
        => httpContextAccessor.HttpContext?.Response.Cookies.Delete(Constants.RefreshToken);
}
