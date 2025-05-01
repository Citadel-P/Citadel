using System.Reflection;
using Application.Features.Networks.Queries;

namespace WebApi.Routes.Endpoints.Resources.Networks;

public sealed record ListNetworksRequest(bool? Dangling = false, string Driver = null, string Id = null, string Name = null)
{
    internal ListNetworks ToQuery(Guid platformId) => new (platformId, Dangling, Driver, Id, Name);

    public static ValueTask<ListNetworksRequest> BindAsync(HttpContext context, ParameterInfo parameter)
    {
        var query = context.Request.Query;

        bool? dangling = null;
        if (query.TryGetValue("dangling", out var danglingStr) &&
            bool.TryParse(danglingStr, out var parsedDangling))
        {
            dangling = parsedDangling;
        }

        var driver = query.TryGetValue("driver", out var driverStr) ? driverStr.ToString() : null;
        var id = query.TryGetValue("id", out var idStr) ? idStr.ToString() : null;
        var name = query.TryGetValue("name", out var nameStr) ? nameStr.ToString() : null;

        var result = new ListNetworksRequest(dangling, driver, id, name);
        return ValueTask.FromResult<ListNetworksRequest>(result);
    }
}
