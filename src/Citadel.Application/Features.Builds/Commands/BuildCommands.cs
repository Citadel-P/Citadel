using Application.Features.Builds.Models;
using Application.Permissions;
using Application.Services.Builds;
using Application.Services.Licensing;
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
            RuleFor(x => x.Project.PlatformId).NotEmpty().When(x => x.Project.BuilderKind == BuildProjectBuilderKind.Platform);
            RuleFor(x => x.Project.BuildAgentPoolId).NotEmpty().When(x => x.Project.BuilderKind == BuildProjectBuilderKind.BuildAgentPool);
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

[RequirePermission(ResourceType.Build, PermissionLevel.Write, ResourceIdProperty = nameof(UpdateBuildProject.ProjectId))]
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

[RequirePermission(ResourceType.Build, PermissionLevel.Write, ResourceIdProperty = nameof(RenameBuildProject.ProjectId))]
public sealed record RenameBuildProject(Guid ProjectId, string Name) : ICommand<Result<BuildProjectResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Write, ResourceIdProperty = nameof(PatchBuildProjectMetadata.ProjectId))]
public sealed record PatchBuildProjectMetadata(Guid ProjectId, string? Description) : ICommand<Result<BuildProjectResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Write, ResourceIdProperty = nameof(ArchiveBuildProject.ProjectId))]
public sealed record ArchiveBuildProject(Guid ProjectId) : ICommand<Result>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read, SpecificPermission.Apply, ResourceIdProperty = nameof(QueueBuildRun.ProjectId))]
public sealed record QueueBuildRun(Guid ProjectId, QueueBuildRunInputModel Input) : ICommand<Result<BuildRunResult>>;

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
    IActivityStreamManager activityStreamManager,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<CreateBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(CreateBuildProject command, CancellationToken cancellationToken)
    {
        var input = command.Project;
        var normalizedName = BuildProject.ToNormalizedName(input.Name);
        if (await unitOfWork.BuildProjects.ExistsByNormalizedNameAsync(normalizedName, cancellationToken))
            return Result.Failure<BuildProjectResult>(new ConflictError("Build project name already exists."));

        if (input.BuilderKind == BuildProjectBuilderKind.BuildAgentPool)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.ElasticBuildExecution,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildProjectResult>(entitlementError);
        }

        if (input.Webhook?.Enabled == true)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildProjectResult>(entitlementError);
        }

        var validation = await ValidateReferencesAsync(
            input.GitRepositoryId,
            input.BuilderKind,
            input.PlatformId,
            input.BuildAgentPoolId,
            input.RegistryId,
            input.BuildSecrets,
            unitOfWork,
            cancellationToken,
            requireBuildPoolEnabled: true);
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
            input.PlatformId ?? Guid.Empty,
            input.RegistryId,
            input.ImageRepository,
            input.TagTemplates,
            input.Webhook,
            input.TimeoutSeconds ?? BuildProject.DefaultTimeoutSeconds,
            input.RetentionRunCount ?? BuildProject.DefaultRetentionRunCount,
            userContextAccessor.Current.ActorId,
            input.BuilderKind,
            input.BuildAgentPoolId);

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
            project.PlatformId == Guid.Empty ? null : project.PlatformId,
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
        BuildProjectBuilderKind builderKind,
        Guid? platformId,
        Guid? buildAgentPoolId,
        Guid registryId,
        IReadOnlyList<BuildSecretSpec>? buildSecrets,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken,
        bool requireBuildPoolEnabled)
    {
        if (await unitOfWork.GitRepositories.GetAsync(gitRepositoryId, cancellationToken) is null)
            return Result.Failure(new NotFoundError("Git repository not found."));

        if (builderKind == BuildProjectBuilderKind.Platform)
        {
            if (!platformId.HasValue || platformId.Value == Guid.Empty)
                return Result.Failure(new BadRequestError("Platform is required."));

            var platform = await unitOfWork.Platforms.GetInfoAsync(platformId.Value, cancellationToken);
            if (platform is null)
                return Result.Failure(new NotFoundError("Platform not found."));
        }
        else
        {
            if (!buildAgentPoolId.HasValue || buildAgentPoolId.Value == Guid.Empty)
                return Result.Failure(new BadRequestError("Build pool is required."));

            var pool = await unitOfWork.BuildAgentPools.GetAsync(buildAgentPoolId.Value, cancellationToken);
            if (pool is null)
                return Result.Failure(new NotFoundError("Build pool not found."));

            if (requireBuildPoolEnabled && !pool.Enabled)
                return Result.Failure(new ConflictError("Build pool is disabled."));
        }

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
    IActivityStreamManager activityStreamManager,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<UpdateBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(UpdateBuildProject command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildProjectResult>(new NotFoundError("Build project not found."));

        var input = command.Project;
        var targetBuilderKind = input.BuilderKind ?? project.BuilderKind;
        var targetBuildAgentPoolId = input.BuildAgentPoolId ?? project.BuildAgentPoolId;
        var selectsExternalPool =
            targetBuilderKind == BuildProjectBuilderKind.BuildAgentPool
            && (project.BuilderKind != BuildProjectBuilderKind.BuildAgentPool
                || targetBuildAgentPoolId != project.BuildAgentPoolId);
        if (selectsExternalPool)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.ElasticBuildExecution,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildProjectResult>(entitlementError);
        }

        if (command.UpdateWebhook && input.Webhook?.Enabled == true)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildProjectResult>(entitlementError);
        }

        var validation = await CreateBuildProjectHandler.ValidateReferencesAsync(
            input.GitRepositoryId ?? project.GitRepositoryId,
            targetBuilderKind,
            input.PlatformId ?? project.PlatformId,
            targetBuildAgentPoolId,
            input.RegistryId ?? project.RegistryId,
            command.UpdateBuildSecrets ? input.BuildSecrets : project.BuildSecrets,
            unitOfWork,
            cancellationToken,
            requireBuildPoolEnabled:
                targetBuilderKind == BuildProjectBuilderKind.BuildAgentPool
                && targetBuildAgentPoolId.HasValue
                && (project.BuilderKind != BuildProjectBuilderKind.BuildAgentPool || targetBuildAgentPoolId.Value != project.BuildAgentPoolId));
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
                input.BuilderKind,
                input.BuildAgentPoolId,
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
            project.PlatformId == Guid.Empty ? null : project.PlatformId,
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
            project.PlatformId == Guid.Empty ? null : project.PlatformId,
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
            project.PlatformId == Guid.Empty ? null : project.PlatformId,
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
    IActivityStreamManager activityStreamManager,
    ILicenseEntitlementService entitlementService)
    : ICommandHandler<QueueBuildRun, Result<BuildRunResult>>
{
    public async ValueTask<Result<BuildRunResult>> Handle(QueueBuildRun command, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(command.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Build project not found."));

        if (!project.Enabled)
            return Result.Failure<BuildRunResult>(new ConflictError("Build project is disabled."));

        if (command.Input.Trigger is BuildRunTrigger.Schedule or BuildRunTrigger.Webhook)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildRunResult>(entitlementError);
        }

        if (project.BuilderKind == BuildProjectBuilderKind.BuildAgentPool)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.ElasticBuildExecution,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<BuildRunResult>(entitlementError);
        }

        if (await unitOfWork.BuildRuns.HasActiveRunAsync(project.Id, cancellationToken))
            return Result.Failure<BuildRunResult>(new ConflictError("Build project already has an active run."));

        var repository = await unitOfWork.GitRepositories.GetAsync(project.GitRepositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<BuildRunResult>(new NotFoundError("Git repository not found."));

        var buildTargetResult = await ResolveBuildTargetAsync(project, unitOfWork, cancellationToken);
        if (!buildTargetResult.IsSuccess(out var buildTarget, out var platformSnapshotError))
            return Result.Failure<BuildRunResult>(platformSnapshotError!);

        if (buildTarget.MaxActiveBuilders.HasValue)
        {
            var activePoolRuns = await unitOfWork.BuildRuns.CountActiveByBuildAgentPoolAsync(buildTarget.PlatformSnapshot.Id, cancellationToken);
            if (activePoolRuns >= buildTarget.MaxActiveBuilders.Value)
            {
                return Result.Failure<BuildRunResult>(
                    new ConflictError($"Build pool \"{buildTarget.PlatformSnapshot.Name}\" has no available builders."));
            }
        }

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
            buildTarget.PlatformSnapshot,
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
            project.PlatformId == Guid.Empty ? null : project.PlatformId,
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

    internal static async Task<Result<BuildTarget>> ResolveBuildTargetAsync(
        BuildProject project,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (project.BuilderKind == BuildProjectBuilderKind.Platform)
        {
            var platform = await unitOfWork.Platforms.GetInfoAsync(project.PlatformId, cancellationToken);
            return platform is null
                ? Result.Failure<BuildTarget>(new NotFoundError("Platform not found."))
                : new BuildTarget(
                    new BuildPlatformSnapshot(platform.Id, platform.Name, platform.Address, platform.ConnectorType),
                    MaxActiveBuilders: null);
        }

        if (project.BuildAgentPoolId is null || project.BuildAgentPoolId.Value == Guid.Empty)
            return Result.Failure<BuildTarget>(new BadRequestError("Build pool is required."));

        var pool = await unitOfWork.BuildAgentPools.GetAsync(project.BuildAgentPoolId.Value, cancellationToken);
        if (pool is null)
            return Result.Failure<BuildTarget>(new NotFoundError("Build pool not found."));

        if (!pool.Enabled)
            return Result.Failure<BuildTarget>(new ConflictError("Build pool is disabled."));

        return pool.ProviderSpec switch
        {
            SelfManagedVmBuildAgentPoolProviderSpec vm => await ResolveSelfManagedBuildTargetAsync(pool, vm, unitOfWork, cancellationToken),
            AwsEc2BuildAgentPoolProviderSpec => Result.Failure<BuildTarget>(
                new ConflictError("AWS EC2 build pool execution is not implemented yet. Use a self-managed Citadel Agent build pool for this runtime slice.")),
            _ => Result.Failure<BuildTarget>(new ConflictError("Build pool provider is unsupported."))
        };
    }

    private static async Task<Result<BuildTarget>> ResolveSelfManagedBuildTargetAsync(
        BuildAgentPool pool,
        SelfManagedVmBuildAgentPoolProviderSpec vm,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (vm.ConnectionMode == BuildAgentPoolConnectionMode.EdgeAgent)
        {
            return new BuildTarget(
                new BuildPlatformSnapshot(
                    pool.Id,
                    pool.Name,
                    $"edge-build-pool://{pool.Id:D}",
                    PlatformConnectorType.EdgeAgent),
                pool.MaxActiveBuilders);
        }

        if (string.IsNullOrWhiteSpace(vm.Endpoint))
            return Result.Failure<BuildTarget>(new BadRequestError("Self-managed VM endpoint is required for this build pool."));

        return new BuildTarget(
            new BuildPlatformSnapshot(
                pool.Id,
                pool.Name,
                vm.Endpoint,
                PlatformConnectorType.Agent),
            pool.MaxActiveBuilders);
    }

    internal sealed record BuildTarget(BuildPlatformSnapshot PlatformSnapshot, int? MaxActiveBuilders);

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
    IPermissionEvaluator permissionEvaluator,
    IBuildRunCoordinator buildRunCoordinator,
    IBuildProjectStreamManager buildProjectStreamManager,
    IBuildRunStreamManager buildRunStreamManager,
    IBuildRunRetentionService buildRunRetentionService)
    : ICommandHandler<CancelBuildRun, Result>
{
    public async ValueTask<Result> Handle(CancelBuildRun command, CancellationToken cancellationToken)
    {
        var existingRun = await unitOfWork.BuildRuns.GetAsync(command.RunId, cancellationToken);
        if (existingRun is null)
            return Result.Failure(new ConflictError("Build run is not active or was not found."));

        var permission = await permissionEvaluator.EvaluateAsync(
            existingRun.BuildProjectId,
            ResourceType.Build,
            cancellationToken);
        if (!permission.Has(PermissionLevel.Read, SpecificPermission.Apply))
            return Result.Failure(new ForbiddenError("Missing permission [Read] with specific [Apply] on [Build]"));

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
            project.BuilderKind,
            project.BuilderKind == BuildProjectBuilderKind.Platform ? project.PlatformId : null,
            project.BuilderKind == BuildProjectBuilderKind.BuildAgentPool ? project.BuildAgentPoolId : null,
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
