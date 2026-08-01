namespace Application.Configs;

public sealed class AutomationOptions
{
    public const string SectionName = "Automations";

    public bool Enabled { get; set; } = true;
    public string DenoPath { get; set; } = "deno";
    public string WorkDir { get; set; } = "./data/automations/runs";
    public string DenoCacheDir { get; set; } = "./data/automations/deno-cache";
    public string InternalBaseUrl { get; set; } = "http://localhost:8000";
    public string? AllowNet { get; set; }
    public int MaxParallelRuns { get; set; } = 4;
    public int DefaultTimeoutSeconds { get; set; } = 300;
    public int MaxTimeoutSeconds { get; set; } = 1800;
    public int MaxLogBytes { get; set; } = 1_048_576;
    public int PollIntervalSeconds { get; set; } = 2;
    public int SchedulePollIntervalSeconds { get; set; } = 30;
}
