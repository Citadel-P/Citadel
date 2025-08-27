namespace Infrastructure.Persistence.Dtos;

internal sealed record PlatformConnectionInfoDto(
    Guid Id,
    string Name,
    string Address,
    string ConnectorType) // PlatformConnectorType 
{
    public PlatformConnectionInfoDto() : this(Guid.Empty, string.Empty, string.Empty, string.Empty)
    {
        
    }
} 
