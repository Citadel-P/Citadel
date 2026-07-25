using System.Diagnostics.CodeAnalysis;
using TimeZoneConverter;

namespace Domain;

public static class TimeZoneResolver
{
    public static TimeZoneInfo Resolve(string timeZoneId)
    {
        if (TryResolve(timeZoneId, out var timeZone))
            return timeZone;

        throw new TimeZoneNotFoundException($"The time zone '{timeZoneId}' was not found.");
    }

    public static bool TryResolve(
        string? timeZoneId,
        [NotNullWhen(true)] out TimeZoneInfo? timeZone)
    {
        if (string.IsNullOrWhiteSpace(timeZoneId))
        {
            timeZone = null;
            return false;
        }

        try
        {
            timeZone = TZConvert.GetTimeZoneInfo(timeZoneId);
            return true;
        }
        catch (TimeZoneNotFoundException)
        {
            timeZone = null;
            return false;
        }
        catch (InvalidTimeZoneException)
        {
            timeZone = null;
            return false;
        }
    }
}
