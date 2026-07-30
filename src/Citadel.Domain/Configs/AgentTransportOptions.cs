namespace Domain.Configs;

public sealed class AgentTransportOptions
{
    public const string SectionName = "AgentTransport";

    public bool AllowInsecure { get; set; } = true;
    public string? CaCertificatePath { get; set; }
}
