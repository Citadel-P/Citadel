namespace Domain.Entities.Alerts;

public class AlertChannel
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public AlertDestination AlertDestination { get; private set; }
    public string Url { get; private set; }
}
