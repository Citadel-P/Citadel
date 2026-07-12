using Domain.Contracts.Resources.Identity;

namespace Application.Features.Identity.Profile;

internal static class ProfileHelpers
{
    internal static string GetSessionDisplayName(string? userAgent)
    {
        if (string.IsNullOrWhiteSpace(userAgent))
            return "Unknown browser";

        var browser = userAgent switch
        {
            var value when value.Contains("Edg/", StringComparison.OrdinalIgnoreCase) => "Edge",
            var value when value.Contains("Chrome/", StringComparison.OrdinalIgnoreCase) => "Chrome",
            var value when value.Contains("Firefox/", StringComparison.OrdinalIgnoreCase) => "Firefox",
            var value when value.Contains("Safari/", StringComparison.OrdinalIgnoreCase) => "Safari",
            var value when value.Contains("curl/", StringComparison.OrdinalIgnoreCase) => "curl",
            _ => "Unknown browser"
        };

        var os = userAgent switch
        {
            var value when value.Contains("Windows", StringComparison.OrdinalIgnoreCase) => "Windows",
            var value when value.Contains("Mac OS X", StringComparison.OrdinalIgnoreCase) => "macOS",
            var value when value.Contains("Android", StringComparison.OrdinalIgnoreCase) => "Android",
            var value when value.Contains("iPhone", StringComparison.OrdinalIgnoreCase)
                || value.Contains("iPad", StringComparison.OrdinalIgnoreCase) => "iOS",
            var value when value.Contains("Linux", StringComparison.OrdinalIgnoreCase) => "Linux",
            _ => null
        };

        return os is null ? browser : $"{browser} on {os}";
    }

    internal static UserSessionSummary ToSummary(this UserSessionRecord record, Guid? currentSessionId)
        => new(
            record.Id,
            GetSessionDisplayName(record.UserAgent),
            record.UserAgent,
            record.IpAddress,
            record.CreatedAt,
            record.LastSeenAt,
            record.ExpiresAt,
            currentSessionId.HasValue && record.Id == currentSessionId.Value);
}
