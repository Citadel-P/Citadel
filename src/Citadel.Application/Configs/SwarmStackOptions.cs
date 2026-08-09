namespace Application.Configs;

public sealed class SwarmStackOptions
{
    public const string SectionName = "SwarmStacks";

    public int RetainedRollbackReleases { get; set; } = 10;
}
