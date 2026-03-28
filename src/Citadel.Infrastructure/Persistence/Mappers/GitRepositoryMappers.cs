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
            onClone: string.IsNullOrWhiteSpace(dto.OnClone) ? null : JsonSerializer.Deserialize(dto.OnClone, GitJsonContext.Default.ListRepoCommand),
            onPull: string.IsNullOrWhiteSpace(dto.OnPull) ? new List<RepoCommand>() : JsonSerializer.Deserialize(dto.OnPull, GitJsonContext.Default.ListRepoCommand));
    }
}
