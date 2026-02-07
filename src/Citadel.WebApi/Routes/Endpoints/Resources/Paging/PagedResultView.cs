namespace WebApi.Routes.Endpoints.Resources.Paging;

public record PagedResultView<T>(IEnumerable<T> Items, int TotalCount, int Page, int PageSize);