using Application.Features.SwarmServices;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record ReplaceDeploymentTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ReplaceStackTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record ReplacePlatformTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Write)]
public sealed record ReplaceGitRepositoryTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Registry, PermissionLevel.Write)]
public sealed record ReplaceRegistryTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.AutomationAction, PermissionLevel.Write)]
public sealed record ReplaceAutomationActionTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.BackupPolicy, PermissionLevel.Write)]
public sealed record ReplaceBackupPolicyTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Write)]
public sealed record ReplaceBuildTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Write)]
public sealed record ReplaceBuildAgentPoolTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record ReplaceSwarmServiceTags(Guid Id, IReadOnlyCollection<Guid>? TagIds) : ICommand<Result<IReadOnlyList<TagSummary>>>;

internal sealed class ReplaceDeploymentTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceDeploymentTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceDeploymentTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.Deployment,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken) is not null,
            "Deployment",
            cancellationToken);
}

internal sealed class ReplaceStackTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceStackTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceStackTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.Stack,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.Stacks.ExistsAsync(command.Id, cancellationToken),
            "Stack",
            cancellationToken);
}

internal sealed class ReplacePlatformTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplacePlatformTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplacePlatformTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.Platform,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.Platforms.ExistsAsync(command.Id, cancellationToken),
            "Platform",
            cancellationToken);
}

internal sealed class ReplaceGitRepositoryTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceGitRepositoryTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceGitRepositoryTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.GitRepository,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken) is not null,
            "Git repository",
            cancellationToken);
}

internal sealed class ReplaceRegistryTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceRegistryTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceRegistryTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.Registry,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.Registries.GetAsync(command.Id, cancellationToken) is not null,
            "Registry",
            cancellationToken);
}

internal sealed class ReplaceAutomationActionTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceAutomationActionTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceAutomationActionTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.AutomationAction,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.AutomationActions.GetAsync(command.Id, cancellationToken) is not null,
            "Automation action",
            cancellationToken);
}

internal sealed class ReplaceBackupPolicyTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceBackupPolicyTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceBackupPolicyTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.BackupPolicy,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.BackupPolicies.GetAsync(command.Id, cancellationToken) is not null,
            "Backup policy",
            cancellationToken);
}

internal sealed class ReplaceBuildTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceBuildTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceBuildTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.Build,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.BuildProjects.GetAsync(command.Id, cancellationToken) is not null,
            "Build project",
            cancellationToken);
}

internal sealed class ReplaceBuildAgentPoolTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext) : ICommandHandler<ReplaceBuildAgentPoolTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceBuildAgentPoolTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.BuildAgentPool,
            command.Id,
            command.TagIds,
            async () => await unitOfWork.BuildAgentPools.GetAsync(command.Id, cancellationToken) is not null,
            "Build pool",
            cancellationToken);
}

internal sealed class ReplaceSwarmServiceTagsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : ICommandHandler<ReplaceSwarmServiceTags, Result<IReadOnlyList<TagSummary>>>
{
    public async ValueTask<Result<IReadOnlyList<TagSummary>>> Handle(ReplaceSwarmServiceTags command, CancellationToken cancellationToken)
        => await ResourceTagReplace.ReplaceAsync(
            unitOfWork,
            userContext,
            TaggableResourceType.SwarmService,
            command.Id,
            command.TagIds,
            async () => await SwarmServiceValidation.ExistsAndCanAccessAsync(
                command.Id,
                unitOfWork,
                userContext,
                cancellationToken),
            "Swarm Service",
            cancellationToken);
}

file static class ResourceTagReplace
{
    internal static async Task<Result<IReadOnlyList<TagSummary>>> ReplaceAsync(
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        TaggableResourceType resourceType,
        Guid resourceId,
        IReadOnlyCollection<Guid>? tagIds,
        Func<Task<bool>> resourceExists,
        string resourceLabel,
        CancellationToken cancellationToken)
    {
        if (!await resourceExists())
            return Result.Failure<IReadOnlyList<TagSummary>>(new NotFoundError($"{resourceLabel} with ID {resourceId} does not exist"));

        var result = await ResourceTagAssignments.ReplaceAsync(
            unitOfWork,
            resourceType,
            resourceId,
            tagIds,
            userContext.Current.ActorId,
            cancellationToken);

        if (result.IsFailure(out var error, out var tags))
            return Result.Failure<IReadOnlyList<TagSummary>>(error);

        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success(tags);
    }
}
