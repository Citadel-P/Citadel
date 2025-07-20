namespace Infrastructure.Persistence.Dtos;

internal sealed record PlatformConnectionInfoDto(
    string Id, // Guid
    string Address,
    string ConnectorType) // PlatformConnectorType 
{
    public PlatformConnectionInfoDto() : this(string.Empty, string.Empty, string.Empty)
    {
        
    }
} 
