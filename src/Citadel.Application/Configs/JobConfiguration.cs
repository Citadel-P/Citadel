namespace Application.Configs;

public class JobConfiguration
{
    public int MonitoringInterval { get; set; } = 15;
    public int BatchSize { get; set; } = 200;
    public int FlashInterval { get; set; } = 120;
}
