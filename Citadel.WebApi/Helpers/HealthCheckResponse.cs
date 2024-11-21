namespace WebApi.Helpers;

using System.Text.Json;
using Microsoft.AspNetCore.Diagnostics.HealthChecks;

internal sealed record HealthCheckResponse(string Status, TimeSpan Duration, HealthCheckEntry[] Entries);

internal sealed record HealthCheckEntry(string Status, string Component, string Description);

internal static class HealthCheckOptionsHelper
{
    /// <summary>
    /// Helper method to override the default health check response
    /// </summary>
    public static HealthCheckOptions GetHealthCheckOptions()
    {
        return new HealthCheckOptions
        {
            ResponseWriter = async (context, report) =>
            {
                context.Response.ContentType = "application/json";
                HealthCheckResponse response = new(
                    report.Status.ToString(),
                    report.TotalDuration,
                    report.Entries.Select(x => new HealthCheckEntry(
                        x.Value.Status.ToString(),
                        x.Key,
                        x.Value.Description)).ToArray());

                await context.Response.WriteAsync(JsonSerializer.Serialize(response));
            }
        };
    }
}