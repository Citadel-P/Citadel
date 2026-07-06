using Domain.Entities.Oidc;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class OidcMappers
{
    internal static OidcProvider ToDomain(this OidcProviderDto dto)
        => OidcProvider.FromPersistence(
            dto.Id,
            dto.Name,
            dto.Description,
            dto.DisplayName,
            dto.Issuer,
            dto.ClientId,
            dto.ClientSecretCiphertext,
            dto.Scopes,
            dto.Enabled,
            dto.AutoProvisionUsers,
            dto.AllowEmailAutoLink,
            dto.RequireEmailVerified,
            dto.AllowedEmailDomains,
            dto.RequiredClaimName,
            dto.RequiredClaimValues,
            dto.DefaultRoleId,
            dto.CreatedByActorId,
            dto.CreatedAt,
            dto.UpdatedAt);
}
