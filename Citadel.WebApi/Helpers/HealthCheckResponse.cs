using System.Text.Json.Serialization;
using Microsoft.AspNetCore.Diagnostics.HealthChecks;

namespace WebApi.Helpers;

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
                    [.. report.Entries.Select(x => new HealthCheckEntry(
                        x.Value.Status.ToString(),
                        x.Key,
                        x.Value.Description))]);

                await context.Response.WriteAsJsonAsync(response, typeof(HealthCheckResponse), HealthCheckSerializerContext.Default, cancellationToken: context.RequestAborted);
            }
        };
    }
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(HealthCheckEntry))]
[JsonSerializable(typeof(HealthCheckResponse))]
internal partial class HealthCheckSerializerContext : JsonSerializerContext 
{
}
