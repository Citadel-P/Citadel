namespace Application.Configs;

public sealed class BackupOptions
{
    public const string SectionName = "Backups";

    public bool Enabled { get; set; } = true;
    public string ResticPath { get; set; } = "restic";
    public string PostgresDumpPath { get; set; } = "pg_dump";
    public string WorkingDirectory { get; set; } = "./data/backups/work";
    public string CoreDataPath { get; set; } = "./data";
    public string[] AllowedCorePaths { get; set; } = ["./data/backups/repositories"];
    public int RepositoryLeaseSeconds { get; set; } = 300;
    public int SourceLeaseSeconds { get; set; } = 300;
    public int PollIntervalSeconds { get; set; } = 2;
    public int SchedulePollIntervalSeconds { get; set; } = 30;
    public int MaxParallelRuns { get; set; } = 2;
    public int DefaultTimeoutSeconds { get; set; } = 120;
    public int MaxLogLineBytes { get; set; } = 8192;
    public int MaxLogBytes { get; set; } = 1_048_576;
}
