using System.Text.Json;
using Domain;
using Domain.Entities.Git;
using Infrastructure.Persistence.Dtos;

namespace Infrastructure.Persistence.Mappers;

internal static class GitAccountMappers
{
    internal static IEnumerable<GitAccount> ToDomain(this IEnumerable<GitAccountDto> dtos)
        => dtos.Select(ToDomain);

    internal static GitAccount ToDomain(this GitAccountDto dto)
    {
        return GitAccount.FromPersistence(
            id: dto.Id,
            name: dto.Name,
            domain: dto.Domain,
            transport: Enum.Parse<GitTransport>(dto.Transport),
            authType: Enum.Parse<GitAuthType>(dto.AuthType),
            createdAt: dto.CreatedAt,
            createdByActorId: dto.CreatedByActorId,
            configuration: (GitAuthConfiguration)(JsonSerializer.Deserialize(dto.Configuration, typeof(GitAuthConfiguration), GitJsonContext.Default)
                ?? throw new NotImplementedException($"Git account configuration is missing for git account id {dto.Id}")));
    }
}
