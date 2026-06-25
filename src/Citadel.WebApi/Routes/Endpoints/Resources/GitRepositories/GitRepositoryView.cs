using Application.Permissions;
using Domain;
using Domain.Entities.Git;
using Hosting.Common;
using WebApi.Routes.Endpoints.Resources.Activities;
using WebApi.Routes.Endpoints.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryView(
    Guid Id,
    Guid CreatedByActorId,
    string Name,
    string? Description,
    GitReposStatus Status,
    string Url,
    string? DefaultBranch,
    Guid? GitAccountId,
    GitRepositorySyncMode SyncMode,
    int? SyncIntervalMinutes,
    RepoWebhookConfig? Webhook,
    RepoCommand? OnClone,
    RepoCommand? OnPull,
    DateTime CreatedAt,
    ResourceControlState ControlState,
    LatestActivityView? LatestActivityView,
    ResourceCapabilities? Capabilities = null)
{
    internal static GitRepositoryView Map(GitRepository gitRepository) => new(
        gitRepository.Id,
        gitRepository.CreatedByActorId,
        gitRepository.Name,
        gitRepository.Description,
        gitRepository.Status,
        gitRepository.Url,
        gitRepository.DefaultBranch,
        gitRepository.GitAccountId,
        gitRepository.SyncMode,
        gitRepository.SyncIntervalMinutes,
        gitRepository.Webhook,
        gitRepository.OnClone,
        gitRepository.OnPull,
        gitRepository.CreatedAt,
        gitRepository.ControlState,
        gitRepository.LatestActivityEvent?.Map());

    internal static async Task<GitRepositoryView> Map(GitRepository gitRepository, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(gitRepository.Id, ResourceType.GitRepository);
        return Map(gitRepository) with
        {
            Capabilities = CapabilityMapper.ToResourceCapabilities(permissions)
        };
    }
}


public sealed record GitRepositoryConfigView(
    Guid Id,
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    GitRepositorySyncMode SyncMode,
    int? SyncIntervalMinutes,
    RepoWebhookConfig? Webhook,
    RepoCommand? OnClone,
    RepoCommand? OnPull)
{
    internal static GitRepositoryConfigView Map(GitRepository gitRepository) => new(
        gitRepository.Id,
        gitRepository.Name,
        gitRepository.Description,
        gitRepository.Url,
        gitRepository.DefaultBranch ?? "main",
        gitRepository.GitAccountId,
        gitRepository.SyncMode,
        gitRepository.SyncIntervalMinutes,
        gitRepository.Webhook,
        gitRepository.OnClone,
        gitRepository.OnPull);
}
