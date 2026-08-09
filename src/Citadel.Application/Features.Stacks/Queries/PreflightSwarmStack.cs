using Application.Services;
using Application.Features.Stacks;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record PreflightSwarmStack(
    string Name,
    Guid PlatformId,
    StackSource StackSource,
    StackSpec Spec,
    StackDriftPolicy? DriftPolicy = null)
    : IQuery<Result<SwarmStackCompatibilityReport>>;

internal sealed class PreflightSwarmStackHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IPermissionService permissionService,
    IGitStackMaterializer gitStackMaterializer)
    : IQueryHandler<PreflightSwarmStack, Result<SwarmStackCompatibilityReport>>
{
    public async ValueTask<Result<SwarmStackCompatibilityReport>> Handle(
        PreflightSwarmStack query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
            return Result.Failure<SwarmStackCompatibilityReport>(new NotFoundError("The provided platform does not exist."));

        var user = userContext.Current;
        if (!user.IsAdmin
            && !await unitOfWork.Platforms.CanAccessAsync(user.UserId, query.PlatformId, cancellationToken))
        {
            return Result.Failure<SwarmStackCompatibilityReport>(
                new NotFoundError("The provided platform does not exist or is not accessible."));
        }

        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor swarm)
        {
            return Result.Failure<SwarmStackCompatibilityReport>(
                new BadRequestError("Swarm Stack preflight requires a Docker Swarm platform."));
        }

        if (!IsCompatible(query.StackSource, query.Spec))
        {
            return Result.Failure<SwarmStackCompatibilityReport>(
                new BadRequestError("StackSource does not match the provided StackSpec."));
        }

        var policyIssues = SwarmStackConfigurationPolicy.GetIssues(query.Spec, query.DriftPolicy);
        var platformIssues = GetPlatformIssues(platform.Status, swarm);

        if (query.Spec is ManualStack manual)
        {
            return Merge(
                StackComposeParser.AnalyzeSwarmCompatibility([manual.ComposeFile], manual.BuildImageBindings),
                policyIssues,
                platformIssues);
        }

        var git = (GitStack)query.Spec;
        var validationError = Commands.GitStackSpecValidation.Validate(git);
        if (validationError is not null)
            return Result.Failure<SwarmStackCompatibilityReport>(new BadRequestError(validationError));

        var repository = await unitOfWork.GitRepositories.GetWithAccountAsync(git.GitRepoId, cancellationToken);
        if (repository is null)
            return Result.Failure<SwarmStackCompatibilityReport>(new NotFoundError("The provided Git repository does not exist."));

        if (!user.IsAdmin)
        {
            var permissions = await permissionService.ResolvePermissionsAsync(
                user.UserId,
                ResourceType.GitRepository,
                git.GitRepoId,
                cancellationToken);
            if (!permissions.Has(PermissionLevel.Read, SpecificPermission.None))
            {
                return Result.Failure<SwarmStackCompatibilityReport>(
                    new ForbiddenError("Missing permission [Read] on [GitRepository]."));
            }
        }

        await unitOfWork.CommitAsync(cancellationToken);

        var temporaryStack = Stack.Create(
            string.IsNullOrWhiteSpace(query.Name) ? "swarm-preflight" : query.Name,
            user.ActorId,
            StackSource.Git,
            query.PlatformId,
            git,
            driftPolicy: StackDriftPolicy.Disabled);
        try
        {
            var materialized = await gitStackMaterializer.MaterializeAsync(
                temporaryStack,
                git,
                repository,
                cancellationToken);
            if (!materialized.IsSuccess(out var source))
            {
                return Merge(
                    new SwarmStackCompatibilityReport(
                        false,
                        [new(
                            SwarmStackCompatibilitySeverity.Error,
                            "source.materialization_failed",
                            materialized.Errors.FirstOrDefault()?.Message ?? "The Git Stack source could not be materialized.",
                            "spec")]),
                    policyIssues,
                    platformIssues);
            }

            var inputLimit = StackComposeParser.GetSwarmInputLimitReport(source.SourceComposeFilePaths.Count, 0);
            if (inputLimit is not null)
                return Merge(inputLimit, policyIssues, platformIssues);

            var totalBytes = 0L;
            foreach (var path in source.SourceComposeFilePaths)
            {
                totalBytes += new FileInfo(path).Length;
                inputLimit = StackComposeParser.GetSwarmInputLimitReport(source.SourceComposeFilePaths.Count, totalBytes);
                if (inputLimit is not null)
                    return Merge(inputLimit, policyIssues, platformIssues);
            }

            var composeFiles = new List<string>(source.SourceComposeFilePaths.Count);
            foreach (var path in source.SourceComposeFilePaths)
                composeFiles.Add(await File.ReadAllTextAsync(path, cancellationToken));

            return Merge(
                StackComposeParser.AnalyzeSwarmCompatibility(composeFiles, git.BuildImageBindings),
                policyIssues,
                platformIssues);
        }
        finally
        {
            await gitStackMaterializer.DiscardSnapshotAsync(
                temporaryStack.Id,
                temporaryStack.CurrentStackReleaseId,
                CancellationToken.None);
        }
    }

    private static IReadOnlyList<SwarmStackCompatibilityIssue> GetPlatformIssues(
        PlatformStatus status,
        DockerSwarmPlatformDescriptor descriptor)
    {
        var issues = new List<SwarmStackCompatibilityIssue>();
        if (status != PlatformStatus.Online)
        {
            issues.Add(new(
                SwarmStackCompatibilitySeverity.Error,
                "platform.offline",
                "The selected Swarm platform is offline.",
                "platformId"));
        }

        if (!descriptor.ControlAvailable
            || !string.Equals(descriptor.LocalNodeState, "active", StringComparison.OrdinalIgnoreCase))
        {
            issues.Add(new(
                SwarmStackCompatibilitySeverity.Error,
                "platform.manager_required",
                "The selected connector must target an active Swarm manager.",
                "platformId"));
        }

        return issues;
    }

    private static SwarmStackCompatibilityReport Merge(
        SwarmStackCompatibilityReport report,
        params IReadOnlyList<SwarmStackCompatibilityIssue>[] additions)
    {
        var issues = additions.Aggregate(
            report.Issues.ToList(),
            static (current, next) =>
            {
                current.AddRange(next);
                return current;
            });
        return new(
            issues.All(static issue => issue.Severity != SwarmStackCompatibilitySeverity.Error),
            issues);
    }

    private static bool IsCompatible(StackSource source, StackSpec spec)
        => (source, spec) is (StackSource.WebEditor, ManualStack) or (StackSource.Git, GitStack);
}
