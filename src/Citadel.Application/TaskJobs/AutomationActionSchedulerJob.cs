using Application.Configs;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.TaskJobs;

internal sealed class AutomationActionSchedulerJob(
    IServiceScopeFactory scopeFactory,
    IOptions<AutomationOptions> automationOptions,
    ILogger<AutomationActionSchedulerJob> logger) : BackgroundService
{
    private readonly AutomationOptions options = automationOptions.Value;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        using var timer = new PeriodicTimer(TimeSpan.FromSeconds(Math.Max(5, options.SchedulePollIntervalSeconds)));

        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken))
            {
                if (!options.Enabled)
                    continue;

                await QueueDueScheduledRunsAsync(stoppingToken);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private async Task QueueDueScheduledRunsAsync(CancellationToken stoppingToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var queueService = scope.ServiceProvider.GetRequiredService<IAutomationRunQueueService>();
        var actions = await unitOfWork.AutomationActions.GetScheduledAsync(stoppingToken);
        var nowUtc = TruncateToMinute(DateTime.UtcNow);

        foreach (var action in actions)
        {
            try
            {
                if (!CronSchedule.IsDue(action.ScheduleCron, action.ScheduleTimeZone, nowUtc))
                    continue;

                if (!await unitOfWork.AutomationActions.TryMarkScheduledAsync(action.Id, nowUtc, stoppingToken))
                    continue;

                await unitOfWork.CommitAsync(stoppingToken);
                await queueService.QueueAsync(
                    action.Id,
                    ActionRunTrigger.Schedule,
                    argsJson: null,
                    timeoutSeconds: null,
                    triggeredByActorId: null,
                    requireEnabled: true,
                    stoppingToken);
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Failed to queue scheduled automation action {ActionId}", action.Id);
                await unitOfWork.RollbackAsync();
            }
        }
    }

    private static DateTime TruncateToMinute(DateTime value)
        => new(value.Year, value.Month, value.Day, value.Hour, value.Minute, 0, DateTimeKind.Utc);
}

internal static class CronSchedule
{
    public static bool IsDue(string? expression, string? timeZoneId, DateTime nowUtc)
    {
        if (string.IsNullOrWhiteSpace(expression))
            return false;

        var fields = expression.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (fields.Length != 5)
            return false;

        var timeZone = ResolveTimeZone(timeZoneId);
        var local = TimeZoneInfo.ConvertTimeFromUtc(nowUtc, timeZone);

        return Matches(fields[0], local.Minute, 0, 59)
            && Matches(fields[1], local.Hour, 0, 23)
            && Matches(fields[2], local.Day, 1, 31)
            && Matches(fields[3], local.Month, 1, 12)
            && MatchesDayOfWeek(fields[4], local.DayOfWeek);
    }

    private static TimeZoneInfo ResolveTimeZone(string? timeZoneId)
    {
        if (string.IsNullOrWhiteSpace(timeZoneId))
            return TimeZoneInfo.Utc;

        try
        {
            return TimeZoneInfo.FindSystemTimeZoneById(timeZoneId);
        }
        catch
        {
            return TimeZoneInfo.Utc;
        }
    }

    private static bool MatchesDayOfWeek(string field, DayOfWeek dayOfWeek)
    {
        var value = (int)dayOfWeek;
        return Matches(field, value, 0, 7) || (value == 0 && Matches(field, 7, 0, 7));
    }

    private static bool Matches(string field, int value, int min, int max)
    {
        foreach (var part in field.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (MatchesPart(part, value, min, max))
                return true;
        }

        return false;
    }

    private static bool MatchesPart(string part, int value, int min, int max)
    {
        var step = 1;
        var rangePart = part;
        var stepIndex = part.IndexOf('/', StringComparison.Ordinal);
        if (stepIndex >= 0)
        {
            rangePart = part[..stepIndex];
            if (!int.TryParse(part[(stepIndex + 1)..], out step) || step <= 0)
                return false;
        }

        var (start, end) = rangePart switch
        {
            "*" or "" => (min, max),
            _ when rangePart.Contains('-', StringComparison.Ordinal) => ParseRange(rangePart),
            _ when int.TryParse(rangePart, out var single) => (single, single),
            _ => (int.MinValue, int.MinValue)
        };

        if (start < min || end > max || start > end)
            return false;

        return value >= start && value <= end && (value - start) % step == 0;
    }

    private static (int Start, int End) ParseRange(string range)
    {
        var parts = range.Split('-', 2, StringSplitOptions.TrimEntries);
        return parts.Length == 2 && int.TryParse(parts[0], out var start) && int.TryParse(parts[1], out var end)
            ? (start, end)
            : (int.MinValue, int.MinValue);
    }
}
