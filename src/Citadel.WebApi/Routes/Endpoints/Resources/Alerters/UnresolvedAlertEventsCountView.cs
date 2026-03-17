namespace WebApi.Routes.Endpoints.Resources.Alerters;

public sealed record UnresolvedAlertEventsCountView(int Count)
{
    public static UnresolvedAlertEventsCountView Map(int count) => new(count);
}
