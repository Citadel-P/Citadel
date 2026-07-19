using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Builds;

internal interface IBuildImageResolver
{
    Task<Result<ResolvedBuildImage>> ResolveLatestAsync(Guid buildProjectId, CancellationToken cancellationToken);
}

internal sealed class BuildImageResolver(IServiceScopeFactory scopeFactory) : IBuildImageResolver
{
    public async Task<Result<ResolvedBuildImage>> ResolveLatestAsync(Guid buildProjectId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var project = await unitOfWork.BuildProjects.GetAsync(buildProjectId, cancellationToken, includeArchived: true);
        if (project is null)
            return Result.Failure<ResolvedBuildImage>(new NotFoundError("Build project not found."));

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
            ImageReference: imageReference,
            Digest: run.ImageDigest,
            RunId: run.Id);
    }
}

internal sealed record ResolvedBuildImage(
    Guid ProjectId,
    string ProjectName,
    Guid RegistryId,
    string ImageReference,
    string? Digest,
    Guid RunId);
