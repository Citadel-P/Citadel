using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Stacks;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

internal interface IManualStackDeployedImageResolver
{
    Task<Result<IReadOnlyDictionary<string, string>>> ResolveAsync(
        Stack stack,
        IReadOnlyList<ManualStackImageCheck> checks,
        CancellationToken cancellationToken);
}

internal sealed class ManualStackDeployedImageResolver(
    IPlatformContainerCache platformCache,
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory) : IManualStackDeployedImageResolver
{
    public async Task<Result<IReadOnlyDictionary<string, string>>> ResolveAsync(
        Stack stack,
        IReadOnlyList<ManualStackImageCheck> checks,
        CancellationToken cancellationToken)
    {
        var release = stack.CurrentStackRelease;
        if (release is null
            || !platformCache.TryGetCacheEntry(release.PlatformId, out var platform, out _))
        {
            return Result.Failure<IReadOnlyDictionary<string, string>>(
                new ConflictError("The stack platform is unavailable."));
        }

        var containerConnector = containerConnectorFactory.GetConnector(platform.ConnectorType);
        var containersResult = await containerConnector.ListContainersAsync(
            StackContainerOwnership.CreateOwnedContainerFilter(
                platform.Address,
                StackProjectNameResolver.Resolve(stack),
                stack.Id),
            cancellationToken);
        if (containersResult.IsFailure(out _, out var containers)
            || containers.Count == 0)
        {
            containersResult = await containerConnector.ListContainersAsync(
                StackContainerOwnership.CreateComposeProjectContainerFilter(
                    platform.Address,
                    StackProjectNameResolver.Resolve(stack)),
                cancellationToken);
        }

        if (containersResult.IsFailure(out _, out containers))
        {
            return Result.Failure<IReadOnlyDictionary<string, string>>(
                new BadGatewayError("Citadel could not inspect the stack's deployed containers."));
        }

        var imageConnector = imageConnectorFactory.GetConnector(platform.ConnectorType);
        var imagesResult = await imageConnector.ListImagesAsync(platform.Address, cancellationToken);
        if (imagesResult.IsFailure(out _, out var images))
        {
            return Result.Failure<IReadOnlyDictionary<string, string>>(
                new BadGatewayError("Citadel could not inspect the platform's deployed images."));
        }

        var imagesById = images
            .Where(static image => !string.IsNullOrWhiteSpace(image.Id))
            .GroupBy(static image => image.Id, StringComparer.OrdinalIgnoreCase)
            .ToDictionary(
                static group => group.Key,
                static group => group.Last(),
                StringComparer.OrdinalIgnoreCase);
        var deployedContainers = containers.Values
            .Where(static container =>
                !string.IsNullOrWhiteSpace(container.ImageId)
                && !string.IsNullOrWhiteSpace(container.Image))
            .ToArray();
        var resolved = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

        foreach (var check in checks)
        {
            var digests = deployedContainers
                .Select(container => (
                    Container: container,
                    Image: imagesById.GetValueOrDefault(container.ImageId)))
                .Where(candidate => candidate.Image is not null
                                    && (ImageReferencesEqual(candidate.Container.Image, check.ImageName)
                                        || ImageMatches(candidate.Image, check.ImageName)))
                .SelectMany(candidate => GetMatchingDigests(candidate.Image!, check.ImageName))
                .Distinct(StringComparer.OrdinalIgnoreCase)
                .ToArray();

            if (digests.Length != 1)
            {
                return Result.Failure<IReadOnlyDictionary<string, string>>(
                    new ConflictError(
                        $"Citadel could not determine the deployed registry digest for service '{check.ServiceName}'. Redeploy the stack with image pulling enabled, then check again."));
            }

            resolved[ManualStackUpdateEvaluator.StateKey(check.ServiceName, check.ImageName)] = digests[0];
        }

        return Result.Success<IReadOnlyDictionary<string, string>>(resolved);
    }

    private static bool ImageMatches(ImageResult image, string expectedImage)
        => (image.RepoTags ?? []).Any(tag => ImageReferencesEqual(tag, expectedImage));

    private static IEnumerable<string> GetMatchingDigests(ImageResult image, string expectedImage)
    {
        var candidates = (image.RepoDigests ?? [])
            .Select(ParseRepoDigest)
            .Where(static candidate => candidate is not null)
            .Select(static candidate => candidate!.Value)
            .ToArray();
        var expectedRepository = NormalizeRepository(RemoveTag(expectedImage));
        var matching = candidates
            .Where(candidate => string.Equals(
                NormalizeRepository(candidate.Repository),
                expectedRepository,
                StringComparison.OrdinalIgnoreCase))
            .Select(static candidate => candidate.Digest)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();

        if (matching.Length > 0)
        {
            return matching;
        }

        var distinct = candidates
            .Select(static candidate => candidate.Digest)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();
        return distinct.Length == 1 ? distinct : [];
    }

    private static (string Repository, string Digest)? ParseRepoDigest(string value)
    {
        var separator = value.LastIndexOf('@');
        if (separator <= 0 || separator == value.Length - 1)
        {
            return null;
        }

        var digest = value[(separator + 1)..];
        return digest.Contains(':')
            ? (value[..separator], digest)
            : null;
    }

    private static bool ImageReferencesEqual(string left, string right)
        => string.Equals(
            NormalizeTaggedReference(left),
            NormalizeTaggedReference(right),
            StringComparison.OrdinalIgnoreCase);

    private static string NormalizeTaggedReference(string value)
    {
        var digestSeparator = value.LastIndexOf('@');
        if (digestSeparator > 0)
        {
            value = value[..digestSeparator];
        }

        var repository = RemoveTag(value);
        var tagSeparator = value.LastIndexOf(':');
        var slash = value.LastIndexOf('/');
        var tag = tagSeparator > slash ? value[(tagSeparator + 1)..] : "latest";
        return $"{NormalizeRepository(repository)}:{tag}";
    }

    private static string RemoveTag(string value)
    {
        var digestSeparator = value.LastIndexOf('@');
        if (digestSeparator > 0)
        {
            value = value[..digestSeparator];
        }

        var slash = value.LastIndexOf('/');
        var tagSeparator = value.LastIndexOf(':');
        return tagSeparator > slash ? value[..tagSeparator] : value;
    }

    private static string NormalizeRepository(string value)
    {
        var repository = value.Trim().TrimStart('/').ToLowerInvariant();
        var slash = repository.IndexOf('/');
        if (slash < 0)
        {
            return $"docker.io/library/{repository}";
        }

        var firstSegment = repository[..slash];
        if (!firstSegment.Contains('.')
            && !firstSegment.Contains(':')
            && !string.Equals(firstSegment, "localhost", StringComparison.Ordinal))
        {
            return $"docker.io/{repository}";
        }

        if (string.Equals(firstSegment, "index.docker.io", StringComparison.Ordinal))
        {
            return "docker.io" + repository[slash..];
        }

        return repository;
    }
}
