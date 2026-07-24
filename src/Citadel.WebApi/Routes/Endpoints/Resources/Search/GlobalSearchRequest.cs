using Microsoft.AspNetCore.Mvc;

namespace WebApi.Routes.Endpoints.Resources.Search;

public sealed record GlobalSearchRequest(
    [FromQuery(Name = "q")] string Query,
    [FromQuery(Name = "types")] string Types = null,
    [FromQuery(Name = "limitPerType")] int LimitPerType = 5);
