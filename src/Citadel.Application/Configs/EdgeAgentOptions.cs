namespace Application.Configs;

public sealed class EdgeAgentOptions
{
    private const string DefaultAgentImageRepository = "ghcr.io/citadel-p/citadel.agent";

    public const string SectionName = "EdgeAgent";

    public string AgentImageRepository { get; set; } = DefaultAgentImageRepository;

    public string? AgentImageTag { get; set; }

    public string GetAgentImage()
    {
        var repository = string.IsNullOrWhiteSpace(AgentImageRepository)
            ? DefaultAgentImageRepository
            : AgentImageRepository.Trim();
        var tag = string.IsNullOrWhiteSpace(AgentImageTag)
            ? global::ThisAssembly.AssemblyInformationalVersion
            : AgentImageTag.Trim();

        return $"{repository}:{NormalizeDockerTag(tag)}";
    }

    private static string NormalizeDockerTag(string tag)
    {
        var normalized = tag.Trim();
        var metadataIndex = normalized.IndexOf('+', StringComparison.Ordinal);
        if (metadataIndex >= 0)
        {
            normalized = normalized[..metadataIndex];
        }

        if (normalized.Length > 1 &&
            normalized[0] is 'v' or 'V' &&
            normalized[1] >= '0' &&
            normalized[1] <= '9')
        {
            normalized = normalized[1..];
        }

        Span<char> chars = normalized.Length <= 256
            ? stackalloc char[normalized.Length]
            : new char[normalized.Length];
        var length = 0;
        foreach (var ch in normalized)
        {
            chars[length++] = IsDockerTagChar(ch) ? ch : '-';
        }

        normalized = new string(chars[..length]).Trim('.', '-');
        return string.IsNullOrWhiteSpace(normalized) ? "latest" : normalized;
    }

    private static bool IsDockerTagChar(char ch) =>
        (ch >= 'a' && ch <= 'z') ||
        (ch >= 'A' && ch <= 'Z') ||
        (ch >= '0' && ch <= '9') ||
        ch is '_' or '.' or '-';
}
