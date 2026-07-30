namespace Infrastructure.EdgeAgents;

internal static class EdgeAgentDefaults
{
    public const int ProtocolVersion = 2;
    public const int MaxEnvelopePayloadBytes = 16 * 1024 * 1024;
    public const int MaxConcurrentCommandsPerSession = 16;
    public const int MaxActiveStreamsPerSession = 8;
    public const int OutboundQueueCapacity = 256;
    public const int PendingOutputQueueCapacity = 256;
    public const int MaxCapabilitiesJsonBytes = 16 * 1024;

    public static readonly string[] RequiredCommands =
    [
        "platform.checkHealth",
        "containers.list",
        "containers.logs"
    ];
}
