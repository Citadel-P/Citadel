using Application.Features.Builds.Models;
using Application.Permissions;
using Domain;
using Domain.Entities.Builds;
using Hosting.Common;
using Hosting.Common.Attributes;
using WebApi.Routes.Endpoints.Resources.Identity;
using WebApi.Routes.Endpoints.Resources.Tags;

namespace WebApi.Routes.Endpoints.Resources.Builds;

public sealed record BuildProjectView(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    bool Enabled,
    Guid GitRepositoryId,
    string Branch,
    string ContextPath,
    string DockerfilePath,
    string? Target,
    IReadOnlyList<BuildArgSpec> BuildArgs,
    IReadOnlyList<BuildSecretSpec> BuildSecrets,
    BuildProjectBuilderKind BuilderKind,
    Guid? PlatformId,
    Guid? BuildAgentPoolId,
    Guid RegistryId,
    string ImageRepository,
    IReadOnlyList<string> TagTemplates,
    BuildWebhookConfig? Webhook,
    int TimeoutSeconds,
    int RetentionRunCount,
    Guid? CurrentRunId,
    ResourceControlState ControlState,
    long? ControlStartedAt,
    Guid CreatedByActorId,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    DateTimeOffset? ArchivedAt,
    long RowVersion,
    BuildRunView? LatestRun,
    IReadOnlyList<TagSummaryView> Tags,
    ResourceCapabilities? Capabilities = null)
{
    internal static BuildProjectView Map(BuildProjectResult result) => Map(result.Project, result.LatestRun);

    internal static async Task<BuildProjectView> Map(BuildProjectResult result, IPermissionEvaluator permissionEvaluator)
        => await Map(result.Project, permissionEvaluator, result.LatestRun);

    internal static BuildProjectView Map(BuildProject project)
        => Map(project, latestRun: null);

    internal static BuildProjectView Map(BuildProject project, BuildRun? latestRun)
        => new(
            project.Id,
            project.Name,
            project.NormalizedName,
            project.Description,
            project.Enabled,
            project.GitRepositoryId,
            project.Branch,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            project.BuildSecrets,
            project.BuilderKind,
            project.BuilderKind == BuildProjectBuilderKind.Platform ? project.PlatformId : null,
            project.BuildAgentPoolId,
            project.RegistryId,
            project.ImageRepository,
            project.TagTemplates,
            project.Webhook,
            project.TimeoutSeconds,
            project.RetentionRunCount,
            project.CurrentRunId,
            project.ControlState,
            project.ControlStartedAt,
            project.CreatedByActorId,
            project.CreatedAt,
            project.UpdatedAt,
            project.ArchivedAt,
            project.RowVersion,
            latestRun is null ? null : BuildRunView.Map(latestRun),
            [.. project.Tags.Select(TagSummaryView.Map)]);

    internal static async Task<BuildProjectView> Map(
        BuildProject project,
        IPermissionEvaluator permissionEvaluator,
        BuildRun? latestRun = null)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(project.Id, ResourceType.Build);
        return Map(project, latestRun) with { Capabilities = CapabilityMapper.ToResourceCapabilities(permissions) };
    }
}

public sealed record BuildProjectsView(IReadOnlyList<BuildProjectView> Projects, ResourceCapabilities Capabilities)
{
    internal static async Task<BuildProjectsView> Map(BuildProjectListResult result, IPermissionEvaluator permissionEvaluator)
    {
        var projects = result.Projects;
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.Build);
        if (projects.Count == 0)
            return new BuildProjectsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = projects.Select(static x => x.Id).ToArray();
        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.Build);
        var views = new BuildProjectView[projects.Count];

        for (var i = 0; i < projects.Count; i++)
        {
            var project = projects[i];
            perms.TryGetValue(project.Id, out var meta);
            views[i] = BuildProjectView.Map(project, result.LatestRuns.GetValueOrDefault(project.Id)) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new BuildProjectsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}

public sealed record BuildRunView(
    Guid Id,
    Guid BuildProjectId,
    string ProjectNameSnapshot,
    Guid GitRepositoryId,
    string GitRepositoryNameSnapshot,
    string Branch,
    string? ResolvedCommitSha,
    string ContextPath,
    string DockerfilePath,
    string? Target,
    IReadOnlyList<BuildArgSpec> BuildArgsSnapshot,
    IReadOnlyList<string> BuildSecretIdsSnapshot,
    BuildPlatformSnapshot PlatformSnapshot,
    BuildRegistrySnapshot RegistrySnapshot,
    string ImageRepository,
    IReadOnlyList<string> TagTemplatesSnapshot,
    IReadOnlyList<string> ImageReferences,
    BuildRunTrigger Trigger,
    Guid? TriggerSourceId,
    BuildRunStatus Status,
    string? ImageDigest,
    int TimeoutSeconds,
    DateTimeOffset QueuedAt,
    DateTimeOffset? StartedAt,
    DateTimeOffset? CompletedAt,
    int? ExitCode,
    string? ErrorCode,
    string? ErrorMessage,
    Guid TriggeredByActorId)
{
    internal static BuildRunView Map(BuildRunResult result) => Map(result.Run);

    internal static BuildRunView Map(BuildRun run)
        => new(
            run.Id,
            run.BuildProjectId,
            run.ProjectNameSnapshot,
            run.GitRepositoryId,
            run.GitRepositoryNameSnapshot,
            run.Branch,
            run.ResolvedCommitSha,
            run.ContextPath,
            run.DockerfilePath,
            run.Target,
            run.BuildArgsSnapshot,
            run.BuildSecretIdsSnapshot,
            run.PlatformSnapshot,
            run.RegistrySnapshot,
            run.ImageRepository,
            run.TagTemplatesSnapshot,
            run.ImageReferences,
            run.Trigger,
            run.TriggerSourceId,
            run.Status,
            run.ImageDigest,
            run.TimeoutSeconds,
            run.QueuedAt,
            run.StartedAt,
            run.CompletedAt,
            run.ExitCode,
            run.ErrorCode,
            run.ErrorMessage,
            run.TriggeredByActorId);
}

public sealed record BuildRunsView(IReadOnlyList<BuildRunView> Runs)
{
    internal static BuildRunsView Map(BuildRunListResult result)
        => new([.. result.Runs.Select(BuildRunView.Map)]);
}

public sealed record BuildLogsView(Guid RunId, IReadOnlyList<BuildRunLogEntry> Logs)
{
    internal static BuildLogsView Map(BuildRunLogResult result) => new(result.RunId, result.Logs);
}

public sealed record BuildAgentPoolView(
    Guid Id,
    string Name,
    string NormalizedName,
    string? Description,
    bool Enabled,
    BuildAgentPoolProvider Provider,
    BuildAgentPoolProviderSpec ProviderSpec,
    int MaxActiveBuilders,
    int QueueTimeoutSeconds,
    int ProvisioningTimeoutSeconds,
    int RegistrationTimeoutSeconds,
    int HeartbeatTimeoutSeconds,
    int CleanupTimeoutSeconds,
    int MaximumInstanceLifetimeSeconds,
    int FailureRetentionMinutes,
    BuildAgentPoolValidationStatus LastValidationStatus,
    string? LastValidationMessage,
    DateTimeOffset? LastValidatedAt,
    Guid CreatedByActorId,
    DateTimeOffset CreatedAt,
    DateTimeOffset UpdatedAt,
    DateTimeOffset? ArchivedAt,
    long RowVersion,
    IReadOnlyList<TagSummaryView> Tags,
    ResourceCapabilities? Capabilities = null)
{
    internal static BuildAgentPoolView Map(BuildAgentPoolResult result) => Map(result.Pool);

    internal static async Task<BuildAgentPoolView> Map(BuildAgentPoolResult result, IPermissionEvaluator permissionEvaluator)
        => await Map(result.Pool, permissionEvaluator);

    internal static BuildAgentPoolView Map(BuildAgentPool pool)
        => new(
            pool.Id,
            pool.Name,
            pool.NormalizedName,
            pool.Description,
            pool.Enabled,
            pool.Provider,
            pool.ProviderSpec,
            pool.MaxActiveBuilders,
            pool.QueueTimeoutSeconds,
            pool.ProvisioningTimeoutSeconds,
            pool.RegistrationTimeoutSeconds,
            pool.HeartbeatTimeoutSeconds,
            pool.CleanupTimeoutSeconds,
            pool.MaximumInstanceLifetimeSeconds,
            pool.FailureRetentionMinutes,
            pool.LastValidationStatus,
            pool.LastValidationMessage,
            pool.LastValidatedAt,
            pool.CreatedByActorId,
            pool.CreatedAt,
            pool.UpdatedAt,
            pool.ArchivedAt,
            pool.RowVersion,
            [.. pool.Tags.Select(TagSummaryView.Map)]);

    internal static async Task<BuildAgentPoolView> Map(BuildAgentPool pool, IPermissionEvaluator permissionEvaluator)
    {
        var permissions = await permissionEvaluator.EvaluateAsync(pool.Id, ResourceType.BuildAgentPool);
        return Map(pool) with { Capabilities = CapabilityMapper.ToResourceCapabilities(permissions) };
    }
}

public sealed record BuildAgentPoolsView(IReadOnlyList<BuildAgentPoolView> Pools, ResourceCapabilities Capabilities)
{
    internal static async Task<BuildAgentPoolsView> Map(BuildAgentPoolListResult result, IPermissionEvaluator permissionEvaluator)
    {
        var pools = result.Pools;
        var resourcesPerms = await permissionEvaluator.EvaluateAsync(ResourceType.BuildAgentPool);
        if (pools.Count == 0)
            return new BuildAgentPoolsView([], CapabilityMapper.ToResourceCapabilities(resourcesPerms));

        var ids = pools.Select(static x => x.Id).ToArray();
        var perms = await permissionEvaluator.EvaluateAsync(ids, ResourceType.BuildAgentPool);
        var views = new BuildAgentPoolView[pools.Count];

        for (var i = 0; i < pools.Count; i++)
        {
            var pool = pools[i];
            perms.TryGetValue(pool.Id, out var meta);
            views[i] = BuildAgentPoolView.Map(pool) with
            {
                Capabilities = CapabilityMapper.ToResourceCapabilities(meta == default ? PermissionMetadata.Empty : meta)
            };
        }

        return new BuildAgentPoolsView(views, CapabilityMapper.ToResourceCapabilities(resourcesPerms));
    }
}
