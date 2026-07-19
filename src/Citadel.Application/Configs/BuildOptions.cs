namespace Application.Configs;

public sealed class BuildOptions
{
    public const string SectionName = "Builds";

    public bool RunCleanupEnabled { get; set; } = true;
    public int RunRetentionDays { get; set; } = 90;
}
