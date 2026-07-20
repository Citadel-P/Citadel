using Application.Features.Builds.Models;
using Application.Services.Builds;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Activities;
using Domain.Entities.Builds;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Builds.Commands;

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record CreateBuildProject(BuildProjectInputModel Project) : ICommand<Result<BuildProjectResult>>
{
    internal sealed class Validator : AbstractValidator<CreateBuildProject>
    {
        public Validator()
        {
            RuleFor(x => x.Project.Name).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Project.Description).MaximumLength(600).When(x => x.Project.Description is not null);
            RuleFor(x => x.Project.GitRepositoryId).NotEmpty();
            RuleFor(x => x.Project.PlatformId).NotEmpty();
            RuleFor(x => x.Project.RegistryId).NotEmpty();
            RuleFor(x => x.Project.ImageRepository).NotEmpty().MaximumLength(512);
            RuleForEach(x => x.Project.BuildSecrets!)
                .SetValidator(new BuildSecretSpecValidator())
                .When(x => x.Project.BuildSecrets is not null);
            RuleFor(x => x.Project.BuildSecrets)
                .Must(BuildProjectCommandValidation.HaveUniqueBuildSecretIds)
                .WithMessage("BuildKit secret ids must be unique.")
                .When(x => x.Project.BuildSecrets is not null);
        }
    }
}

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record UpdateBuildProject(
    Guid ProjectId,
    UpdateBuildProjectInputModel Project,
    bool UpdateDescription,
    bool UpdateBuildArgs,
    bool UpdateBuildSecrets,
    bool UpdateWebhook) : ICommand<Result<BuildProjectResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateBuildProject>
    {
        public Validator()
        {
            RuleFor(x => x.ProjectId).NotEmpty();
            RuleFor(x => x.Project.Description).MaximumLength(600).When(x => x.Project.Description is not null);
            RuleFor(x => x.Project.ImageRepository).MaximumLength(512).When(x => x.Project.ImageRepository is not null);
            RuleForEach(x => x.Project.BuildSecrets!)
                .SetValidator(new BuildSecretSpecValidator())
                .When(x => x.UpdateBuildSecrets && x.Project.BuildSecrets is not null);
            RuleFor(x => x.Project.BuildSecrets)
                .Must(BuildProjectCommandValidation.HaveUniqueBuildSecretIds)
                .WithMessage("BuildKit secret ids must be unique.")
                .When(x => x.UpdateBuildSecrets && x.Project.BuildSecrets is not null);
        }
    }
}

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record RenameBuildProject(Guid ProjectId, string Name) : ICommand<Result<BuildProjectResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record PatchBuildProjectMetadata(Guid ProjectId, string? Description) : ICommand<Result<BuildProjectResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record ArchiveBuildProject(Guid ProjectId) : ICommand<Result>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record QueueBuildRun(Guid ProjectId, QueueBuildRunInputModel Input) : ICommand<Result<BuildRunResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read, SpecificPermission.Apply)]
public sealed record CancelBuildRun(Guid RunId) : ICommand<Result>;

internal sealed class BuildSecretSpecValidator : AbstractValidator<BuildSecretSpec>
{
    public BuildSecretSpecValidator()
    {
        RuleFor(x => x.Id)
            .Must(BuildProject.IsBuildKitSecretId)
            .WithMessage("BuildKit secret id must use only letters, numbers, '.', '_' or '-'.");
        RuleFor(x => x.SecretId).NotEmpty();
    }
}

file static class BuildProjectCommandValidation
{
    public static bool HaveUniqueBuildSecretIds(IReadOnlyList<BuildSecretSpec>? secrets)
    {
        if (secrets is null || secrets.Count == 0)
            return true;

        var ids = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var secret in secrets)
        {
            if (!BuildProject.IsBuildKitSecretId(secret.Id))
                continue;

            if (!ids.Add(secret.Id.Trim()))
                return false;
        }

        return true;
    }
}

internal sealed class CreateBuildProjectHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildProjectStreamManager buildProjectStreamManager,
    IActivityStreamManager activityStreamManager)
    : ICommandHandler<CreateBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(CreateBuildProject command, CancellationToken cancellationToken)
    {
        var input = command.Project;
        var normalizedName = BuildProject.ToNormalizedName(input.Name);
        if (await unitOfWork.BuildProjects.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BuildProjectResult>(new ConflictError("Build project name already exists."));

        var validation = await ValidateReferencesAsync(
            input.GitRepositoryId,
            input.PlatformId,
            input.RegistryId,
            input.BuildSecrets,
            unitOfWork,
            cancellationToken);
        if (validation.IsFailure(out var validationError))
            return Result.Failure<BuildProjectResult>(validationError);

        var repository = await unitOfWork.GitRepositories.GetAsync(input.GitRepositoryId, cancellationToken);
        var project = new BuildProject(
            input.Name,
            input.Description,
            input.Enabled,
            input.GitRepositoryId,
            input.Branch ?? repository?.DefaultBranch ?? "main",
            input.ContextPath ?? ".",
            input.DockerfilePath ?? "Dockerfile",
            input.Target,
            input.BuildArgs,
            input.BuildSecrets,
            input.PlatformId,
            input.RegistryId,
            input.ImageRepository,
            input.TagTemplates,
            input.Webhook,
            input.TimeoutSeconds ?? BuildProject.DefaultTimeoutSeconds,
            input.RetentionRunCount ?? BuildProject.DefaultRetentionRunCount,
            userContextAccessor.Current.ActorId);

        try
        {
            project.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<BuildProjectResult>(new BadRequestError(ex.Message));
        }

        var rows = await unitOfWork.BuildProjects.AddAsync(project, cancellationToken, input.TagIds, userContextAccessor.Current.ActorId);
        if (rows == 0)
            return Result.Failure<BuildProjectResult>(new BadRequestError("One or more tags do not exist."));

        var activity = BuildActivity.Create(
            project,
            project.PlatformId,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildCreated,
            new BuildCreated(BuildActivity.Snapshot(project)));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildProjectStreamManager.SendBuildProjectInfo(project, "create");
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        return Result.Success(new BuildProjectResult(project));
    }

    internal static async Task<Result> ValidateReferencesAsync(
        Guid gitRepositoryId,
        Guid platformId,
        Guid registryId,
        IReadOnlyList<BuildSecretSpec>? buildSecrets,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.GitRepositories.GetAsync(gitRepositoryId, cancellationToken) is null)
            return Result.Failure(new NotFoundError("Git repository not found."));

        var platform = await unitOfWork.Platforms.GetInfoAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure(new NotFoundError("Platform not found."));

        if (await unitOfWork.Registries.GetAsync(registryId, cancellationToken) is null)
            return Result.Failure(new NotFoundError("Registry not found."));

        foreach (var secret in buildSecrets ?? [])
        {
            if (await unitOfWork.SecretDefinitions.GetAsync(secret.SecretId, cancellationToken) is null)
                return Result.Failure(new NotFoundError($"Build secret '{secret.Id}' not found."));
        }

        return Result.Success();
    }
}

internal sealed class UpdateBuildProjectHandler(
    IUnitOfWork unitOfWork,
    IBuildProjectStreamManager buildProjectStreamManager,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityStreamManager)
    : ICommandHandler<UpdateBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(UpdateBuildProject command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildProjectResult>(new NotFoundError("Build project not found."));

        var input = command.Project;
        var validation = await CreateBuildProjectHandler.ValidateReferencesAsync(
            input.GitRepositoryId ?? project.GitRepositoryId,
            input.PlatformId ?? project.PlatformId,
            input.RegistryId ?? project.RegistryId,
            command.UpdateBuildSecrets ? input.BuildSecrets : project.BuildSecrets,
            unitOfWork,
            cancellationToken);
        if (validation.IsFailure(out var validationError))
            return Result.Failure<BuildProjectResult>(validationError);

        var oldSnapshot = BuildActivity.Snapshot(project);
        try
        {
            project.Update(
                input.Description,
                input.Enabled,
                input.GitRepositoryId,
                input.Branch,
                input.ContextPath,
                input.DockerfilePath,
                input.Target,
                input.BuildArgs,
                command.UpdateBuildArgs,
                input.BuildSecrets,
                command.UpdateBuildSecrets,
                input.PlatformId,
                input.RegistryId,
                input.ImageRepository,
                input.TagTemplates,
                input.Webhook,
                command.UpdateWebhook,
                input.TimeoutSeconds,
                input.RetentionRunCount);
        }
        catch (Exception ex) when (ex is ArgumentException or InvalidOperationException)
        {
            return Result.Failure<BuildProjectResult>(new BadRequestError(ex.Message));
        }

        var activity = BuildActivity.Create(
            project,
            project.PlatformId,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildUpdated,
            new BuildUpdated(oldSnapshot, BuildActivity.Snapshot(project)));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildProjects.UpdateAsync(project, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildProjectStreamManager.SendBuildProjectInfo(project);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        return Result.Success(new BuildProjectResult(project));
    }
}

internal sealed class RenameBuildProjectHandler(
    IUnitOfWork unitOfWork,
    IBuildProjectStreamManager buildProjectStreamManager,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityStreamManager)
    : ICommandHandler<RenameBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(RenameBuildProject command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildProjectResult>(new NotFoundError("Build project not found."));

        var normalizedName = BuildProject.ToNormalizedName(command.Name);
        if (await unitOfWork.BuildProjects.ExistsByNormalizedNameExceptAsync(normalizedName, project.Id, cancellationToken))
            return Result.Failure<BuildProjectResult>(new ConflictError("Build project name already exists."));

        var oldName = project.Name;
        project.Rename(command.Name);
        var activity = BuildActivity.Create(
            project,
            project.PlatformId,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildRenamed,
            new BuildRenamed(oldName, project.Name));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildProjects.UpdateAsync(project, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildProjectStreamManager.SendBuildProjectInfo(project);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        return Result.Success(new BuildProjectResult(project));
    }
}

internal sealed class PatchBuildProjectMetadataHandler(
    IUnitOfWork unitOfWork,
    IBuildProjectStreamManager buildProjectStreamManager)
    : ICommandHandler<PatchBuildProjectMetadata, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(PatchBuildProjectMetadata command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildProjectResult>(new NotFoundError("Build project not found."));

        project.UpdateDescription(command.Description);
        await unitOfWork.BuildProjects.UpdateAsync(project, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildProjectStreamManager.SendBuildProjectInfo(project);
        return Result.Success(new BuildProjectResult(project));
    }
}

internal sealed class ArchiveBuildProjectHandler(
    IUnitOfWork unitOfWork,
    IBuildProjectStreamManager buildProjectStreamManager,
    IUserContextAccessor userContextAccessor,
    IActivityStreamManager activityStreamManager)
    : ICommandHandler<ArchiveBuildProject, Result>
{
    public async ValueTask<Result> Handle(ArchiveBuildProject command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure(new NotFoundError("Build project not found."));

        if (await unitOfWork.BuildRuns.HasActiveRunAsync(project.Id, cancellationToken))
            return Result.Failure(new ConflictError("Build project has an active run."));

        var activity = BuildActivity.Create(
            project,
            project.PlatformId,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildDeleted,
            new BuildDeleted(BuildActivity.Snapshot(project)));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.BuildProjects.ArchiveAsync(project.Id, DateTimeOffset.UtcNow, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildProjectStreamManager.SendBuildProjectInfo(project, "delete");
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        return Result.Success();
    }
}

internal sealed class QueueBuildRunHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IActivityStreamManager activityStreamManager)
    : ICommandHandler<QueueBuildRun, Result<BuildRunResult>>
{
    public async ValueTask<Result<BuildRunResult>> Handle(QueueBuildRun command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Build project not found."));

        if (!project.Enabled)
            return Result.Failure<BuildRunResult>(new ConflictError("Build project is disabled."));

        if (await unitOfWork.BuildRuns.HasActiveRunAsync(project.Id, cancellationToken))
            return Result.Failure<BuildRunResult>(new ConflictError("Build project already has an active run."));

        var repository = await unitOfWork.GitRepositories.GetAsync(project.GitRepositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Git repository not found."));

        var platform = await unitOfWork.Platforms.GetInfoAsync(project.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Platform not found."));

        var registry = await unitOfWork.Registries.GetAsync(project.RegistryId, cancellationToken);
        if (registry is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Registry not found."));

        var imageReferences = ResolveImageReferences(registry.RegistryHost, project.ImageRepository, project.TagTemplates, project.Branch, null);
        var run = new BuildRun(
            project.Id,
            project.Name,
            repository.Id,
            repository.Name,
            project.Branch,
            null,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.BuildArgs,
            [.. project.BuildSecrets.Select(static s => s.Id)],
            new BuildPlatformSnapshot(platform.Id, platform.Name, platform.Address, platform.ConnectorType),
            new BuildRegistrySnapshot(registry.Id, registry.Name, registry.RegistryHost),
            project.ImageRepository,
            project.TagTemplates,
            imageReferences,
            command.Input.Trigger,
            command.Input.TriggerSourceId,
            userContextAccessor.Current.ActorId,
            project.TimeoutSeconds);

        var marked = await unitOfWork.BuildProjects.MarkProcessingAsync(project.Id, run.Id, cancellationToken);
        if (marked == 0)
            return Result.Failure<BuildRunResult>(new ConflictError("Build project already has an active run."));

        await unitOfWork.BuildRuns.AddAsync(run, cancellationToken);
        var activity = BuildActivity.Create(
            project,
            project.PlatformId,
            userContextAccessor.Current.ActorId,
            ActivityEventType.BuildRunQueued,
            new BuildRunQueued(run.Id, run.Trigger));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run, "create");
        var updatedProject = await unitOfWork.BuildProjects.GetAsync(project.Id, cancellationToken);
        if (updatedProject is not null)
            await buildProjectStreamManager.SendBuildProjectInfo(updatedProject, latestRun: run);
        await activityStreamManager.SendActivityInfo(await activity.AssignActor(unitOfWork, cancellationToken));
        return Result.Success(new BuildRunResult(run));
    }

    internal static IReadOnlyList<string> ResolveImageReferences(
        string registryHost,
        string imageRepository,
        IReadOnlyList<string> tagTemplates,
        string branch,
        string? commitSha)
    {
        var host = registryHost.Replace("https://", "", StringComparison.OrdinalIgnoreCase)
            .Replace("http://", "", StringComparison.OrdinalIgnoreCase)
            .TrimEnd('/');
        var shortSha = string.IsNullOrWhiteSpace(commitSha) ? "pending" : commitSha[..Math.Min(12, commitSha.Length)];
        var safeBranch = SanitizeDockerTagPart(branch, "branch");
        return [.. tagTemplates.Select(template =>
        {
            var tag = template
                .Replace("{branch}", safeBranch, StringComparison.OrdinalIgnoreCase)
                .Replace("{shortSha}", shortSha, StringComparison.OrdinalIgnoreCase)
                .Replace("{sha}", commitSha ?? "pending", StringComparison.OrdinalIgnoreCase);
            return $"{host}/{imageRepository}:{SanitizeDockerTag(tag)}";
        })];
    }

    private static string SanitizeDockerTagPart(string value, string fallback)
    {
        var sanitized = new string(value
            .Select(static ch => char.IsLetterOrDigit(ch) || ch is '_' or '.' or '-' ? ch : '-')
            .ToArray())
            .Trim('.', '-');

        return string.IsNullOrWhiteSpace(sanitized) ? fallback : sanitized;
    }

    private static string SanitizeDockerTag(string value)
    {
        var tag = SanitizeDockerTagPart(value, "build");
        if (!char.IsLetterOrDigit(tag[0]) && tag[0] != '_')
            tag = "b" + tag;

        return tag.Length <= 128 ? tag : tag[..128];
    }
}

internal sealed class CancelBuildRunHandler(
    IUnitOfWork unitOfWork,
    IBuildRunCoordinator buildRunCoordinator,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IBuildRunRetentionService buildRunRetentionService)
    : ICommandHandler<CancelBuildRun, Result>
{
    public async ValueTask<Result> Handle(CancelBuildRun command, CancellationToken cancellationToken)
    {
        var wasRegistered = buildRunCoordinator.Cancel(command.RunId);

        var run = await unitOfWork.BuildRuns.CancelQueuedOrRunningAsync(
            command.RunId,
            DateTimeOffset.UtcNow,
            "Build run cancelled.",
            cancellationToken);
        if (run is null)
        {
            if (!wasRegistered)
                buildRunCoordinator.Unregister(command.RunId);
            return Result.Failure(new ConflictError("Build run is not active or was not found."));
        }
        if (!wasRegistered)
            buildRunCoordinator.Unregister(command.RunId);

        await unitOfWork.BuildProjects.MarkIdleAsync(run.BuildProjectId, run.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await buildRunStreamManager.SendBuildRunInfo(run);
        await buildRunRetentionService.PruneAsync(run.BuildProjectId, cancellationToken);
        var project = await unitOfWork.BuildProjects.GetAsync(run.BuildProjectId, cancellationToken);
        if (project is not null)
            await buildProjectStreamManager.SendBuildProjectInfo(project, latestRun: run);
        return Result.Success();
    }
}

internal sealed class ExecuteQueuedBuildRunHandler(IBuildRunExecutionService executionService)
    : ICommandHandler<ExecuteQueuedBuildRun, Result>
{
    public async ValueTask<Result> Handle(ExecuteQueuedBuildRun command, CancellationToken cancellationToken)
        => await executionService.ExecuteAsync(command.RunId, cancellationToken);
}

public sealed record ExecuteQueuedBuildRun(Guid RunId) : ICommand<Result>;

internal static class BuildActivity
{
    internal static ActivityEvent Create(
        BuildProject project,
        Guid? platformId,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info)
        => new(
            platformId: platformId,
            resourceId: project.Id,
            actorId: actorId,
            resourceName: project.Name,
            eventType: eventType,
            status: ActivityStatus.Success,
            info: info);

    internal static BuildProjectSnapshot Snapshot(BuildProject project)
        => new(
            project.Id,
            project.Name,
            project.Description,
            project.Enabled,
            project.GitRepositoryId,
            project.Branch,
            project.ContextPath,
            project.DockerfilePath,
            project.Target,
            project.PlatformId,
            project.RegistryId,
            project.ImageRepository,
            project.TagTemplates,
            SanitizeWebhook(project.Webhook),
            project.TimeoutSeconds,
            project.RetentionRunCount,
            project.BuildSecrets);

    private static BuildWebhookConfig? SanitizeWebhook(BuildWebhookConfig? webhook)
        => webhook is null || string.IsNullOrEmpty(webhook.Secret)
            ? webhook
            : webhook with { Secret = null };
}
