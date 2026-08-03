using Application.Features.Containers.Queries;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Mediator;
using System.Security.Cryptography;
using System.Text;
using YamlDotNet.Core;

namespace Application.Features.Stacks.Queries;

public sealed record ComposeProjectRuntimeService(
    string Name,
    string? Image,
    int ContainerCount,
    IReadOnlyList<ContainerStateStatus> States);

public sealed record ComposeProjectImportSource(
    Guid PlatformId,
    string PlatformName,
    string ProjectName,
    IReadOnlyList<string> ContainerIds,
    IReadOnlyList<string> ContainerNames,
    IReadOnlyList<ComposeProjectRuntimeService> Services);

public sealed record ComposeProjectStackDraft(
    string Name,
    Guid PlatformId,
    string? Description,
    StackDriftPolicy DriftPolicy,
    IReadOnlyCollection<Guid> TagIds);

public sealed record ComposeProjectImportDraft(
    ComposeProjectImportSource Source,
    ComposeProjectStackDraft Draft,
    IReadOnlyCollection<AdoptionIssue> Issues,
    string RuntimeFingerprint);

public sealed record ComposeProjectServiceComparison(
    string Name,
    int RuntimeContainerCount,
    string? RuntimeImage,
    bool DefinedInSource,
    string? SourceImage);

public sealed record ComposeProjectImportValidation(
    IReadOnlyList<ComposeProjectServiceComparison> Services,
    IReadOnlyCollection<AdoptionIssue> Issues,
    string PreviewFingerprint);

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record GetComposeProjectImportDraft(Guid PlatformId, string ProjectName)
    : IQuery<Result<ComposeProjectImportDraft>>;

internal sealed class GetComposeProjectImportDraftHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IAdoptionFingerprintService fingerprintService)
    : IQueryHandler<GetComposeProjectImportDraft, Result<ComposeProjectImportDraft>>
{
    public async ValueTask<Result<ComposeProjectImportDraft>> Handle(
        GetComposeProjectImportDraft query,
        CancellationToken cancellationToken)
    {
        var contextResult = await ComposeProjectImportDraftFactory.LoadContextAsync(
            query.PlatformId,
            query.ProjectName,
            unitOfWork,
            connectorFactory,
            containerAuthorizationService,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<ComposeProjectImportDraft>(contextResult.Errors);

        var name = await GetAvailableNameAsync(query.ProjectName, cancellationToken);
        return ComposeProjectImportDraftFactory.Create(context, name, fingerprintService);
    }

    private async Task<string> GetAvailableNameAsync(
        string projectName,
        CancellationToken cancellationToken)
    {
        var baseName = ContainerAdoptionDraftFactory.NormalizeName(projectName);
        if (!await unitOfWork.Stacks.ExistsAsync(baseName, cancellationToken))
            return baseName;

        for (var suffix = 2; suffix <= 100; suffix++)
        {
            var suffixText = $"-{suffix}";
            var candidate = $"{baseName[..Math.Min(baseName.Length, 64 - suffixText.Length)].TrimEnd('-', '_')}{suffixText}";
            if (!await unitOfWork.Stacks.ExistsAsync(candidate, cancellationToken))
                return candidate;
        }

        return $"{baseName[..Math.Min(baseName.Length, 55)].TrimEnd('-', '_')}-{Guid.NewGuid():N}"[..64];
    }
}

internal sealed record ComposeProjectContainerContext(
    Container Container,
    ContainerInspectionInfo Inspection,
    string ServiceName,
    bool IsOneOff);

internal sealed record ComposeProjectImportContext(
    Platform Platform,
    string ProjectName,
    IReadOnlyList<ComposeProjectContainerContext> Containers,
    Guid? OrphanedOwnerStackId = null)
{
    public IReadOnlyList<ComposeProjectContainerContext> ManagedContainers { get; } =
        Containers.Where(static container => !container.IsOneOff).ToArray();
}

internal sealed record ComposeProjectSourceAnalysis(
    StackSpec SafeSpec,
    IReadOnlyDictionary<string, StackComposeService> Services,
    string SourceDigest);

internal static class ComposeProjectImportDraftFactory
{
    private const int MaxConcurrentInspections = 4;
    private const string ComposeImageLabel = "com.docker.compose.image";
    private const string ComposeOneOffLabel = "com.docker.compose.oneoff";

    internal static async Task<Result<ComposeProjectImportContext>> LoadContextAsync(
        Guid platformId,
        string projectName,
        IUnitOfWork unitOfWork,
        IConnectorFactory<IContainerConnector> connectorFactory,
        IContainerAuthorizationService containerAuthorizationService,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(projectName))
            return Result.Failure<ComposeProjectImportContext>(new BadRequestError("Compose project name is required."));
        if (projectName.Length > 128)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new BadRequestError("Compose project name cannot exceed 128 characters."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null)
            return Result.Failure<ComposeProjectImportContext>(new NotFoundError("Platform does not exist."));
        if (platform.PlatformDescriptor.Type != PlatformType.Docker)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new BadRequestError("Only Compose projects on Docker Standalone platforms can be imported as stacks."));
        }
        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<ComposeProjectImportContext>(new ConflictError("Platform is offline."));

        await unitOfWork.CommitAsync(cancellationToken);

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var runtimeResult = await connector.ListContainersAsync(
            StackContainerOwnership.CreateComposeProjectContainerFilter(platform.Address, projectName),
            cancellationToken);
        if (!runtimeResult.IsSuccess(out var runtimeContainers))
            return Result.Failure<ComposeProjectImportContext>(runtimeResult.Errors);

        var runtime = runtimeContainers.Values
            .Where(container => string.Equals(container.Stack, projectName, StringComparison.Ordinal))
            .OrderBy(container => container.Id, StringComparer.Ordinal)
            .ToArray();
        if (runtime.Length == 0)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new NotFoundError($"Compose project '{projectName}' has no containers."));
        }

        var persisted = (await unitOfWork.Containers.GetByIdsAsync(
                [.. runtime.Select(container => container.Id)],
                cancellationToken))
            .ToDictionary(container => container.DockerContainerId, StringComparer.OrdinalIgnoreCase);
        if (persisted.Count != runtime.Length || runtime.Any(container => !persisted.ContainsKey(container.Id)))
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("Compose project containers have not finished synchronizing. Refresh the platform and try again."));
        }

        var containers = runtime.Select(container => persisted[container.Id]).ToArray();
        if (containers.Any(container => container.IsSystem))
            return Result.Failure<ComposeProjectImportContext>(new ConflictError("System containers cannot be imported."));
        if (containers.Any(container => container.DeploymentId is not null || container.StackId is not null))
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("One or more Compose project containers are already managed by Citadel."));
        }
        if (containers.Any(container => container.ControlState == ResourceControlState.Processing))
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("One or more Compose project containers are currently processing another operation."));
        }
        if (containers.Any(container => !string.Equals(container.DockerStack, projectName, StringComparison.Ordinal)))
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("Compose project membership changed. Refresh the container list."));
        }

        var hasAccess = await containerAuthorizationService.HasAccessAsync(
            [.. containers.Select(container => container.DockerContainerId)],
            ResourceType.Platform,
            PermissionLevel.Read,
            SpecificPermission.Inspect,
            cancellationToken);
        if (!hasAccess)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ForbiddenError("Missing specific permission [Inspect] on [Platform]."));
        }

        await unitOfWork.CommitAsync(cancellationToken);

        var inspected = new (Container Container, Result<ContainerInspectionInfo> Result)[containers.Length];
        var nextIndex = -1;

        async Task InspectWorkerAsync()
        {
            while (true)
            {
                var index = Interlocked.Increment(ref nextIndex);
                if (index >= containers.Length)
                    return;

                var container = containers[index];
                inspected[index] = (
                    container,
                    await connector.InspectAsync(
                        new InspectContainerCommand(platform.Address, container.DockerContainerId),
                        cancellationToken));
            }
        }

        var workers = new Task[Math.Min(MaxConcurrentInspections, containers.Length)];
        for (var index = 0; index < workers.Length; index++)
            workers[index] = InspectWorkerAsync();

        await Task.WhenAll(workers);
        var contexts = new List<ComposeProjectContainerContext>(inspected.Length);
        Guid? orphanedOwnerStackId = null;
        var ownedContainerCount = 0;
        foreach (var item in inspected)
        {
            if (!item.Result.IsSuccess(out var inspection))
                return Result.Failure<ComposeProjectImportContext>(item.Result.Errors);
            if (!string.Equals(inspection.Id, item.Container.DockerContainerId, StringComparison.OrdinalIgnoreCase))
            {
                return Result.Failure<ComposeProjectImportContext>(
                    new ConflictError("Container identity changed while preparing the import draft."));
            }
            var labels = inspection.Config?.Labels;
            if (HasCitadelOwnershipLabel(labels))
            {
                ownedContainerCount++;
                if (labels is null || !StackContainerOwnership.TryGetStackId(labels, out var ownerStackId))
                {
                    return Result.Failure<ComposeProjectImportContext>(
                        new ConflictError(
                            $"Container '{item.Container.Name}' has invalid or unsupported Citadel ownership labels."));
                }
                if (orphanedOwnerStackId is not null && orphanedOwnerStackId != ownerStackId)
                {
                    return Result.Failure<ComposeProjectImportContext>(
                        new ConflictError("Compose project containers reference different Citadel owners."));
                }

                orphanedOwnerStackId = ownerStackId;
            }

            var serviceName = GetServiceName(inspection);
            var isOneOff = IsOneOff(inspection);
            contexts.Add(new ComposeProjectContainerContext(item.Container, inspection, serviceName, isOneOff));
        }

        if (ownedContainerCount > 0 && ownedContainerCount != inspected.Length)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("Compose project contains a mix of Citadel-owned and unmanaged containers."));
        }
        if (orphanedOwnerStackId is not null
            && await unitOfWork.Stacks.ExistsAsync(orphanedOwnerStackId.Value, cancellationToken))
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("Compose project is owned by an existing Citadel stack."));
        }

        var context = new ComposeProjectImportContext(
            platform,
            projectName,
            contexts.OrderBy(container => container.Container.DockerContainerId, StringComparer.Ordinal).ToArray(),
            orphanedOwnerStackId);
        if (context.ManagedContainers.Count == 0)
        {
            return Result.Failure<ComposeProjectImportContext>(
                new ConflictError("The Compose project has no long-running service containers to import."));
        }

        return context;
    }

    internal static ComposeProjectImportDraft Create(
        ComposeProjectImportContext context,
        string name,
        IAdoptionFingerprintService fingerprintService)
    {
        var issues = GetRuntimeIssues(context);
        var services = GetRuntimeServices(context);
        return new ComposeProjectImportDraft(
            new ComposeProjectImportSource(
                context.Platform.Id,
                context.Platform.Name,
                context.ProjectName,
                [.. context.ManagedContainers.Select(container => container.Container.DockerContainerId)],
                [.. context.ManagedContainers.Select(container => container.Container.Name)],
                services),
            new ComposeProjectStackDraft(
                name,
                context.Platform.Id,
                $"Imported from Docker Compose project {context.ProjectName}.",
                StackDriftPolicy.Disabled,
                []),
            issues,
            ComputeRuntimeFingerprint(context, fingerprintService));
    }

    internal static ComposeProjectImportValidation CreateValidation(
        ComposeProjectImportContext context,
        ComposeProjectSourceAnalysis source,
        IAdoptionFingerprintService fingerprintService)
    {
        var issues = GetRuntimeIssues(context).ToList();
        var runtimeServices = GetRuntimeServices(context);
        var comparisons = runtimeServices
            .Select(runtime =>
            {
                source.Services.TryGetValue(runtime.Name, out var desired);
                return new ComposeProjectServiceComparison(
                    runtime.Name,
                    runtime.ContainerCount,
                    runtime.Image,
                    desired is not null,
                    desired?.Image);
            })
            .ToList();

        foreach (var desired in source.Services.Values.OrderBy(service => service.ServiceName, StringComparer.Ordinal))
        {
            if (runtimeServices.Any(runtime => string.Equals(runtime.Name, desired.ServiceName, StringComparison.OrdinalIgnoreCase)))
                continue;

            comparisons.Add(new ComposeProjectServiceComparison(
                desired.ServiceName,
                0,
                null,
                true,
                desired.Image));
            issues.Add(new AdoptionIssue(
                "SOURCE_SERVICE_NOT_RUNNING",
                $"Source service '{desired.ServiceName}' has no current project container. A future Apply may create it.",
                AdoptionIssueSeverity.Warning));
        }

        var matched = comparisons.Count(comparison =>
            comparison.DefinedInSource && comparison.RuntimeContainerCount > 0);
        if (matched == 0)
        {
            issues.Add(new AdoptionIssue(
                "NO_MATCHING_SERVICES",
                "The selected source does not define any detected long-running Compose service.",
                AdoptionIssueSeverity.Blocker));
        }

        foreach (var comparison in comparisons.Where(comparison =>
                     comparison.RuntimeContainerCount > 0 && !comparison.DefinedInSource))
        {
            issues.Add(new AdoptionIssue(
                "RUNTIME_SERVICE_NOT_IN_SOURCE",
                $"Runtime service '{comparison.Name}' is not defined by the selected source. A future Apply may remove it.",
                AdoptionIssueSeverity.Warning));
        }

        foreach (var comparison in comparisons.Where(comparison =>
                     comparison.RuntimeContainerCount > 0
                     && comparison.DefinedInSource
                     && !string.IsNullOrWhiteSpace(comparison.RuntimeImage)
                     && !string.IsNullOrWhiteSpace(comparison.SourceImage)
                     && !string.Equals(comparison.RuntimeImage, comparison.SourceImage, StringComparison.OrdinalIgnoreCase)))
        {
            issues.Add(new AdoptionIssue(
                "SERVICE_IMAGE_DIFFERS",
                $"Service '{comparison.Name}' currently uses '{comparison.RuntimeImage}', while the selected source defines '{comparison.SourceImage}'.",
                AdoptionIssueSeverity.Warning));
        }

        return new ComposeProjectImportValidation(
            comparisons.OrderBy(comparison => comparison.Name, StringComparer.OrdinalIgnoreCase).ToArray(),
            issues,
            ComputePreviewFingerprint(context, source.SourceDigest, fingerprintService));
    }

    internal static async Task<Result<ComposeProjectSourceAnalysis>> AnalyzeSourceAsync(
        ComposeProjectImportContext context,
        string stackName,
        StackSource stackSource,
        StackSpec spec,
        IUnitOfWork unitOfWork,
        IGitStackMaterializer gitStackMaterializer,
        IPermissionService permissionService,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        var safeSpec = NormalizeImportSpec(context.ProjectName, spec);
        if (!IsCompatible(stackSource, safeSpec))
        {
            return Result.Failure<ComposeProjectSourceAnalysis>(
                new BadRequestError("Stack source does not match the provided stack specification."));
        }

        if (safeSpec is ManualStack manual)
        {
            return AnalyzeComposeFiles(
                safeSpec,
                [manual.ComposeFile],
                ComputeSourceDigest(safeSpec, "web", [manual.ComposeFile]));
        }

        var git = (GitStack)safeSpec;
        var validationError = Commands.GitStackSpecValidation.Validate(git);
        if (validationError is not null)
            return Result.Failure<ComposeProjectSourceAnalysis>(new BadRequestError(validationError));

        var repository = await unitOfWork.GitRepositories.GetWithAccountAsync(git.GitRepoId, cancellationToken);
        if (repository is null)
            return Result.Failure<ComposeProjectSourceAnalysis>(new NotFoundError("The provided Git repository does not exist."));

        var user = userContext.Current;
        if (!user.IsAdmin)
        {
            var permissions = await permissionService.ResolvePermissionsAsync(
                user.UserId,
                ResourceType.GitRepository,
                git.GitRepoId,
                cancellationToken);
            if (!permissions.Has(PermissionLevel.Read, SpecificPermission.None))
            {
                return Result.Failure<ComposeProjectSourceAnalysis>(
                    new ForbiddenError("Missing permission [Read] on [GitRepository]."));
            }
        }

        await unitOfWork.CommitAsync(cancellationToken);

        var temporaryStack = Stack.Create(
            stackName,
            user.ActorId,
            StackSource.Git,
            context.Platform.Id,
            safeSpec,
            driftPolicy: StackDriftPolicy.Disabled);
        try
        {
            var materialized = await gitStackMaterializer.MaterializeAsync(
                temporaryStack,
                git,
                repository,
                cancellationToken);
            if (!materialized.IsSuccess(out var source))
                return Result.Failure<ComposeProjectSourceAnalysis>(materialized.Errors);

            var composeFiles = new List<string>(source.SourceComposeFilePaths.Count);
            foreach (var path in source.SourceComposeFilePaths)
                composeFiles.Add(await File.ReadAllTextAsync(path, cancellationToken));

            var sourceDigest = ComputeSourceDigest(
                safeSpec,
                $"git:{source.ResolvedCommitSha}",
                composeFiles);
            return AnalyzeComposeFiles(safeSpec, composeFiles, sourceDigest);
        }
        catch (YamlException)
        {
            return Result.Failure<ComposeProjectSourceAnalysis>(
                new BadRequestError("The selected Compose source is not valid YAML."));
        }
        finally
        {
            await gitStackMaterializer.DiscardSnapshotAsync(
                temporaryStack.Id,
                temporaryStack.CurrentStackReleaseId,
                CancellationToken.None);
        }
    }

    internal static string ComputeRuntimeFingerprint(
        ComposeProjectImportContext context,
        IAdoptionFingerprintService fingerprintService)
    {
        var builder = new StringBuilder(context.Containers.Count * 65);
        builder.Append(context.Platform.Id).Append('|').Append(context.ProjectName).Append('|');
        foreach (var container in context.Containers)
        {
            builder.Append(ContainerAdoptionDraftFactory.ComputeFingerprint(
                container.Container,
                container.Inspection,
                fingerprintService));
            builder.Append('|').Append(container.ServiceName).Append('|').Append(container.IsOneOff);
        }

        return fingerprintService.Sign(builder.ToString());
    }

    internal static string ComputePreviewFingerprint(
        ComposeProjectImportContext context,
        string sourceDigest,
        IAdoptionFingerprintService fingerprintService)
        => fingerprintService.Sign(
            $"{ComputeRuntimeFingerprint(context, fingerprintService)}|{sourceDigest}");

    private static Result<ComposeProjectSourceAnalysis> AnalyzeComposeFiles(
        StackSpec safeSpec,
        IReadOnlyList<string> composeFiles,
        string sourceDigest)
    {
        if (composeFiles.Count == 0 || composeFiles.All(string.IsNullOrWhiteSpace))
        {
            return Result.Failure<ComposeProjectSourceAnalysis>(
                new BadRequestError("At least one non-empty Compose file is required."));
        }

        try
        {
            var services = composeFiles.Count == 1
                ? StackComposeParser.ParseServices(Guid.Empty, Guid.Empty, composeFiles[0])
                : StackComposeParser.ParseServices(Guid.Empty, Guid.Empty, composeFiles);
            if (services.Count == 0)
            {
                return Result.Failure<ComposeProjectSourceAnalysis>(
                    new BadRequestError("The selected Compose source does not define any services."));
            }

            return new ComposeProjectSourceAnalysis(safeSpec, services, sourceDigest);
        }
        catch (YamlException)
        {
            return Result.Failure<ComposeProjectSourceAnalysis>(
                new BadRequestError("The selected Compose source is not valid YAML."));
        }
    }

    internal static StackSpec NormalizeImportSpec(string projectName, StackSpec spec)
        => spec switch
        {
            ManualStack manual => manual with
            {
                ProjectName = projectName,
                DestroyBeforeDeploy = false
            },
            GitStack git => git with
            {
                ProjectName = projectName,
                DestroyBeforeDeploy = false
            },
            _ => spec
        };

    private static bool IsCompatible(StackSource source, StackSpec spec)
        => (source, spec) is (StackSource.WebEditor, ManualStack) or (StackSource.Git, GitStack);

    private static IReadOnlyList<ComposeProjectRuntimeService> GetRuntimeServices(
        ComposeProjectImportContext context)
        => context.ManagedContainers
            .Where(container => !string.IsNullOrWhiteSpace(container.ServiceName))
            .GroupBy(container => container.ServiceName, StringComparer.OrdinalIgnoreCase)
            .Select(group => new ComposeProjectRuntimeService(
                group.Key,
                group.Select(GetRuntimeImage).FirstOrDefault(image => !string.IsNullOrWhiteSpace(image)),
                group.Count(),
                [.. group.Select(container => container.Container.State)]))
            .OrderBy(service => service.Name, StringComparer.OrdinalIgnoreCase)
            .ToArray();

    private static IReadOnlyList<AdoptionIssue> GetRuntimeIssues(ComposeProjectImportContext context)
    {
        var issues = new List<AdoptionIssue>();
        foreach (var container in context.ManagedContainers.Where(container => string.IsNullOrWhiteSpace(container.ServiceName)))
        {
            issues.Add(new AdoptionIssue(
                "COMPOSE_SERVICE_MISSING",
                $"Container '{container.Container.Name}' has no Compose service label.",
                AdoptionIssueSeverity.Blocker));
        }

        foreach (var group in context.ManagedContainers
                     .Where(container => !string.IsNullOrWhiteSpace(container.ServiceName))
                     .GroupBy(container => container.ServiceName, StringComparer.OrdinalIgnoreCase)
                     .Where(group => group.Count() > 1))
        {
            issues.Add(new AdoptionIssue(
                "SCALED_SERVICE",
                $"Service '{group.Key}' currently has {group.Count()} containers. Citadel will associate all of them.",
                AdoptionIssueSeverity.Warning));
        }

        foreach (var container in context.Containers.Where(container => container.IsOneOff))
        {
            issues.Add(new AdoptionIssue(
                "ONE_OFF_CONTAINER_EXCLUDED",
                $"One-off container '{container.Container.Name}' will remain unmanaged.",
                AdoptionIssueSeverity.Warning));
        }

        return issues;
    }

    private static string? GetRuntimeImage(ComposeProjectContainerContext container)
        => container.Inspection.Config?.Labels.TryGetValue(ComposeImageLabel, out var image) == true
            ? image
            : container.Inspection.Config?.Image;

    private static string GetServiceName(ContainerInspectionInfo inspection)
        => inspection.Config?.Labels.TryGetValue(ComposeLabels.Service, out var service) == true
            ? service
            : string.Empty;

    private static bool IsOneOff(ContainerInspectionInfo inspection)
        => inspection.Config?.Labels.TryGetValue(ComposeOneOffLabel, out var oneOff) == true
           && string.Equals(oneOff, "True", StringComparison.OrdinalIgnoreCase);

    private static bool HasCitadelOwnershipLabel(IReadOnlyDictionary<string, string>? labels)
        => labels is not null
           && (labels.Keys.Any(label =>
                   label.StartsWith(CitadelLabels.Prefix, StringComparison.OrdinalIgnoreCase)
                   || label.StartsWith(CitadelLabels.LegacyExtensionPrefix, StringComparison.OrdinalIgnoreCase))
               || StackContainerOwnership.IsCitadelManaged(labels));

    private static string ComputeSourceDigest(
        StackSpec spec,
        string sourceIdentity,
        IReadOnlyList<string> composeFiles)
    {
        var builder = new StringBuilder();
        Append(builder, sourceIdentity);
        Append(builder, spec.ProjectName);
        Append(builder, spec.EnvFilePath);
        Append(builder, spec.RegistryId);
        Append(builder, spec.DestroyBeforeDeploy);
        AppendCommand(builder, spec.PreDeploy);
        AppendCommand(builder, spec.PostDeploy);

        foreach (var binding in (spec.BuildImageBindings ?? [])
                     .OrderBy(binding => binding.ServiceName, StringComparer.OrdinalIgnoreCase)
                     .ThenBy(binding => binding.BuildProjectId))
        {
            Append(builder, binding.ServiceName);
            Append(builder, binding.BuildProjectId);
            Append(builder, binding.RedeployOnBuild);
        }

        switch (spec)
        {
            case ManualStack manual:
                Append(builder, manual.UpdateBehavior);
                break;
            case GitStack git:
                Append(builder, git.GitRepoId);
                Append(builder, git.Branch);
                Append(builder, git.CommitSha);
                Append(builder, git.UpdateBehavior);
                Append(builder, git.WorkingDirectory);
                AppendMany(builder, git.ComposePaths);
                AppendMany(builder, git.ComposeEnvFilesFromRepo);
                AppendMany(builder, git.WatchPaths);
                AppendMany(builder, git.AdditionalEnvFileFromRepo);
                break;
        }

        foreach (var composeFile in composeFiles)
            Append(builder, composeFile);

        return Hash(builder.ToString());
    }

    private static void AppendCommand(StringBuilder builder, StackCommand? command)
    {
        Append(builder, command?.Path);
        AppendMany(builder, command?.Commands);
    }

    private static void AppendMany(StringBuilder builder, IEnumerable<string>? values)
    {
        foreach (var value in values ?? [])
            Append(builder, value);
    }

    private static void Append(StringBuilder builder, object? value)
    {
        var text = value?.ToString() ?? string.Empty;
        builder.Append(text.Length).Append(':').Append(text).Append('|');
    }

    private static string Hash(string value)
        => Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(value)));
}
