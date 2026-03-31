using Domain;
using Domain.Entities.Git;
using Infrastructure.Persistence.Dtos;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class GitRepositoryMappers
{
    internal static IEnumerable<GitRepository> ToDomain(this IEnumerable<GitRepositoryDto> dtos)
        => dtos.Select(ToDomain);

    internal static GitRepository ToDomain(this GitRepositoryDto dto)
    {
        return GitRepository.FromPersistence(
            id: dto.Id,
            name: dto.Name,
            description: dto.Description,
            url: dto.Url,
            defaultBranch: dto.DefaultBranch,
            status: Enum.Parse<GitReposStatus>(dto.Status),
            gitAccountId: dto.GitAccountId,
            createdAt: dto.CreatedAt,
            createdByActorId: dto.CreatedByActorId,
            webHookEnabled: dto.WebHookEnabled == 1,
            webHookSecret: dto.WebHookSecret,
            onClone: string.IsNullOrWhiteSpace(dto.OnClone) ? null : JsonSerializer.Deserialize(dto.OnClone, GitJsonContext.Default.RepoCommand),
            onPull: string.IsNullOrWhiteSpace(dto.OnPull) ? null : JsonSerializer.Deserialize(dto.OnPull, GitJsonContext.Default.RepoCommand),
            controlState: string.IsNullOrWhiteSpace(dto.ControlState) ? ResourceControlState.Idle : Enum.Parse<ResourceControlState>(dto.ControlState),
            controlStartedAt: dto.ControlStartedAt,
            controlTriggeredBy: dto.ControlTriggeredBy,
            rowVersion: dto.RowVersion,
            gitAccount: dto.GitAccount_Name == null ? null : GitAccount.FromPersistence(
                id: dto.GitAccountId.Value,
                name: dto.GitAccount_Name,
                domain: dto.GitAccount_Domain,
                transport: Enum.Parse<GitTransport>(dto.GitAccount_Transport),
                authType: Enum.Parse<GitAuthType>(dto.GitAccount_AuthType),
                createdAt: DateTime.MinValue,
                createdByActorId: Guid.Empty,
                configuration: JsonSerializer.Deserialize(dto.GitAccount_Configuration, GitJsonContext.Default.GitAuthConfiguration)));
    }
}
