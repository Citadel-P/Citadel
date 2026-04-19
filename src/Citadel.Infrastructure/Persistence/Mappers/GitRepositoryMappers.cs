using Domain;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Infrastructure.Persistence.Dtos;
using Microsoft.AspNetCore.Http.HttpResults;
using System.Text.Json;

namespace Infrastructure.Persistence.Mappers;

internal static class GitRepositoryMappers
{
    internal static IEnumerable<GitRepository> ToDomain(this IEnumerable<GitRepositoryDto> dtos)
        => dtos.Select(ToDomain);

    internal static GitRepository ToDomain(this GitRepositoryDto dto)
    {
        var hasLinkedAccount = dto.GitAccountId.HasValue
            && !string.IsNullOrWhiteSpace(dto.GitAccount_Name)
            && !string.IsNullOrWhiteSpace(dto.GitAccount_Domain)
            && !string.IsNullOrWhiteSpace(dto.GitAccount_Transport)
            && !string.IsNullOrWhiteSpace(dto.GitAccount_AuthType)
            && !string.IsNullOrWhiteSpace(dto.GitAccount_Configuration);

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
            latestActivityEvent: string.IsNullOrEmpty(dto.ActivityEvent_ActivityEventInfo) ? null : ActivityEvent.FromPersistence(
                id: dto.ActivityEvent_Id.Value,
                platformId: Guid.CreateVersion7(),
                resourceId: Guid.CreateVersion7(),
                resourceName: "N/A",
                createdAt: dto.ActivityEvent_CreatedAt.Value,
                createdByActorId: Guid.CreateVersion7(),
                resourceType: ActivityResourceType.GitRepository,
                info: JsonSerializer.Deserialize(dto.ActivityEvent_ActivityEventInfo, EventInfoJsonContext.Default.ActivityEventInfo),
                status: Enum.Parse<ActivityStatus>(dto.ActivityEvent_Status),
                eventType: Enum.Parse<ActivityEventType>(dto.ActivityEvent_EventType)
                ),
            gitAccount: !hasLinkedAccount ? null : GitAccount.FromPersistence(
                id: dto.GitAccountId.Value,
                name: dto.GitAccount_Name!,
                domain: dto.GitAccount_Domain!,
                transport: Enum.Parse<GitTransport>(dto.GitAccount_Transport!),
                authType: Enum.Parse<GitAuthType>(dto.GitAccount_AuthType!),
                createdAt: DateTime.MinValue,
                createdByActorId: Guid.Empty,
                configuration: JsonSerializer.Deserialize(dto.GitAccount_Configuration!, GitJsonContext.Default.GitAuthConfiguration)));
    }
}
