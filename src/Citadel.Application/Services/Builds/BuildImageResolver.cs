using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Builds;

internal interface IBuildImageResolver
{
    Task<Result<ResolvedBuildImage>> ResolveAsync(
        Guid buildProjectId,
        string? resolvedImageReference,
        string? resolvedDigest,
        Guid? resolvedBuildRunId,
        CancellationToken cancellationToken);
}

internal sealed class BuildImageResolver(IServiceScopeFactory scopeFactory) : IBuildImageResolver
{
    public async Task<Result<ResolvedBuildImage>> ResolveAsync(
        Guid buildProjectId,
        string? resolvedImageReference,
        string? resolvedDigest,
        Guid? resolvedBuildRunId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var project = await unitOfWork.BuildProjects.GetAsync(buildProjectId, cancellationToken, includeArchived: true);
        if (project is null)
            return Result.Failure<ResolvedBuildImage>(new NotFoundError("Build project not found."));

        if (!string.IsNullOrWhiteSpace(resolvedImageReference))
        {
            Guid? retainedRunId = resolvedBuildRunId is Guid runId && runId != Guid.Empty
                ? runId
                : null;
            var registryId = project.RegistryId;
            if (retainedRunId is Guid retainedRun)
            {
                var resolvedRun = await unitOfWork.BuildRuns.GetAsync(retainedRun, cancellationToken);
                if (resolvedRun?.BuildProjectId == project.Id)
                    registryId = resolvedRun.RegistrySnapshot.Id;
            }

            return new ResolvedBuildImage(
                ProjectId: project.Id,
                ProjectName: project.Name,
                RegistryId: registryId,
                ImageReference: BuildImageReference.PinToDigest(resolvedImageReference, resolvedDigest),
                Digest: resolvedDigest,
                RunId: retainedRunId);
        }

        var run = await unitOfWork.BuildRuns.GetLatestSuccessfulByProjectAsync(buildProjectId, cancellationToken);
        if (run is null)
            return Result.Failure<ResolvedBuildImage>("Build has no successful image yet.");

        var imageReference = run.ImageReferences.FirstOrDefault();
        if (string.IsNullOrWhiteSpace(imageReference))
            return Result.Failure<ResolvedBuildImage>("Latest successful build does not have an image reference.");

        return new ResolvedBuildImage(
            ProjectId: project.Id,
            ProjectName: project.Name,
            RegistryId: run.RegistrySnapshot.Id,
            ImageReference: BuildImageReference.PinToDigest(imageReference, run.ImageDigest),
            Digest: run.ImageDigest,
            RunId: run.Id);
    }
}

internal static class BuildImageReference
{
    public static string PinToDigest(string imageReference, string? digest)
    {
        if (string.IsNullOrWhiteSpace(digest))
            return imageReference;

        var reference = imageReference.Trim();
        var digestSeparator = reference.LastIndexOf('@');
        if (digestSeparator >= 0)
            reference = reference[..digestSeparator];
        else
        {
            var lastSlash = reference.LastIndexOf('/');
            var tagSeparator = reference.LastIndexOf(':');
            if (tagSeparator > lastSlash)
                reference = reference[..tagSeparator];
        }

        return $"{reference}@{digest.Trim()}";
    }
}

internal sealed record ResolvedBuildImage(
    Guid ProjectId,
    string ProjectName,
    Guid RegistryId,
    string ImageReference,
    string? Digest,
    Guid? RunId);
