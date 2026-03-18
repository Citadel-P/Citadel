namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record UnresolvedAlertsCountView(int Count)
{
    public static UnresolvedAlertsCountView Map(int count) => new(count);
}
