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
