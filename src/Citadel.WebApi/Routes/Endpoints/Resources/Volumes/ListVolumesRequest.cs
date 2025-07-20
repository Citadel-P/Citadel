using System.Reflection;
using Application.Features.Volumes.Queries;
using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Volumes;

public sealed record ListVolumesRequest(
    [FromQuery] bool? Dangling = null,
    [FromQuery] string Driver = null,
    [FromQuery] string Name = null)
{
    internal ListVolumes ToQuery(Guid platformId) => new(platformId, Dangling, Driver, Name);

    public static ValueTask<ListVolumesRequest> BindAsync(HttpContext context, ParameterInfo parameter)
    {
        var query = context.Request.Query;

        bool? dangling = null;
        if (query.TryGetValue("dangling", out var danglingStr) &&
            bool.TryParse(danglingStr, out var parsedDangling))
        {
            dangling = parsedDangling;
        }
        var driver = query.TryGetValue("driver", out var driverStr) ? driverStr.ToString() : null;
        var name = query.TryGetValue("name", out var nameStr) ? nameStr.ToString() : null;

        var result = new ListVolumesRequest(dangling, driver, name);
        return ValueTask.FromResult(result);
    }
}
