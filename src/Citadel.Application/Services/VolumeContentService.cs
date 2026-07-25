using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Application.Configs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Volumes;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services;

internal sealed class VolumeContentService(
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IVolumeHelperImageResolver helperImageResolver,
    IAgentRuntimeImageResolver agentRuntimeImageResolver,
    ILogger<VolumeContentService> logger)
    : IVolumeContentService
{
    internal const string DevelopmentHelperImage = "citadel.dev";

    private const string HelperLauncherExecutable = "/bin/sh";
    private const string HelperLauncherName = "citadel-volume-helper";
    private const string HelperLauncherScript = """
        for executable in \
          ./Citadel.VolumeHelper \
          ./publish/Citadel.VolumeHelper \
          /app/Citadel.VolumeHelper \
          /app/publish/Citadel.VolumeHelper \
          /src/src/Citadel.VolumeHelper/bin/Release/net*/linux-*/native/Citadel.VolumeHelper \
          /src/src/Citadel.VolumeHelper/bin/Release/net*/linux-*/Citadel.VolumeHelper \
          /src/src/Citadel.VolumeHelper/bin/Debug/net*/Citadel.VolumeHelper \
          /src/Citadel.VolumeHelper/bin/Release/net*/linux-*/native/Citadel.VolumeHelper \
          /src/Citadel.VolumeHelper/bin/Release/net*/linux-*/Citadel.VolumeHelper \
          /src/Citadel.VolumeHelper/bin/Debug/net*/Citadel.VolumeHelper \
          ./Citadel.Agent.VolumeHelper \
          ./publish/Citadel.Agent.VolumeHelper \
          /app/Citadel.Agent.VolumeHelper \
          /app/publish/Citadel.Agent.VolumeHelper \
          /src/src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/native/Citadel.Agent.VolumeHelper \
          /src/src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/Citadel.Agent.VolumeHelper \
          /src/src/Citadel.Agent.VolumeHelper/bin/Debug/net*/Citadel.Agent.VolumeHelper \
          /src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/native/Citadel.Agent.VolumeHelper \
          /src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/Citadel.Agent.VolumeHelper \
          /src/Citadel.Agent.VolumeHelper/bin/Debug/net*/Citadel.Agent.VolumeHelper
        do
          if [ -x "$executable" ]; then
            exec "$executable" "$@"
          fi
        done

        if command -v dotnet >/dev/null 2>&1; then
          for dll in \
            ./Citadel.VolumeHelper.dll \
            ./publish/Citadel.VolumeHelper.dll \
            /app/Citadel.VolumeHelper.dll \
            /app/publish/Citadel.VolumeHelper.dll \
            /src/src/Citadel.VolumeHelper/bin/Release/net*/linux-*/Citadel.VolumeHelper.dll \
            /src/src/Citadel.VolumeHelper/bin/Debug/net*/Citadel.VolumeHelper.dll \
            /src/Citadel.VolumeHelper/bin/Release/net*/linux-*/Citadel.VolumeHelper.dll \
            /src/Citadel.VolumeHelper/bin/Debug/net*/Citadel.VolumeHelper.dll \
            ./Citadel.Agent.VolumeHelper.dll \
            ./publish/Citadel.Agent.VolumeHelper.dll \
            /app/Citadel.Agent.VolumeHelper.dll \
            /app/publish/Citadel.Agent.VolumeHelper.dll \
            /src/src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/Citadel.Agent.VolumeHelper.dll \
            /src/src/Citadel.Agent.VolumeHelper/bin/Debug/net*/Citadel.Agent.VolumeHelper.dll \
            /src/Citadel.Agent.VolumeHelper/bin/Release/net*/linux-*/Citadel.Agent.VolumeHelper.dll \
            /src/Citadel.Agent.VolumeHelper/bin/Debug/net*/Citadel.Agent.VolumeHelper.dll
          do
            if [ -f "$dll" ]; then
              exec dotnet "$dll" "$@"
            fi
          done
        fi

        echo "Citadel volume helper executable was not found in the helper image. Rebuild or pull the matching Citadel helper image." >&2
        exit 127
        """;
    private const string HelperRoot = "/data";
    private const int MaxEntries = 1000;
    private const int MaxListingPayloadBytes = 1024 * 1024;
    private const int MaxHelperLogBytes = 8192;
    private const long HelperMemoryBytes = 128L * 1024 * 1024;
    private static readonly TimeSpan DefaultOperationTimeout = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan HelperStartProbeDelay = TimeSpan.FromMilliseconds(150);
    private static readonly TimeSpan HelperLogTimeout = TimeSpan.FromSeconds(2);

    public async Task<Result<VolumeDirectoryListing>> ListDirectoryAsync(
        ListVolumeDirectoryCommand command,
        CancellationToken cancellationToken)
    {
        if (!IsSupportedConnector(command.ConnectorType))
        {
            return Result.Failure<VolumeDirectoryListing>(
                new BadRequestError("Volume content browsing is not supported by this platform connector."));
        }

        var helper = await CreateAndStartHelperAsync(command, cancellationToken);
        if (!helper.IsSuccess(out var session, out var createError))
        {
            return Result.Failure<VolumeDirectoryListing>(createError);
        }

        await using (session)
        {
            var args = BuildHelperCommand(
                "volume-helper",
                "list",
                "--root",
                HelperRoot,
                "--path",
                command.Path.ApiPath,
                "--max-entries",
                MaxEntries.ToString(),
                "--max-payload-bytes",
                MaxListingPayloadBytes.ToString());

            var helperResult = await RunJsonHelperAsync<VolumeHelperListResponse>(
                session,
                args,
                cancellationToken);

            if (!helperResult.IsSuccess(out var response, out var helperError))
                return Result.Failure<VolumeDirectoryListing>(helperError);

            if (!string.IsNullOrWhiteSpace(response.ErrorCode))
                return Result.Failure<VolumeDirectoryListing>(MapHelperError(response.ErrorCode, response.ErrorMessage));

            return new VolumeDirectoryListing(
                command.PlatformId,
                command.VolumeName,
                response.Path,
                response.Entries
                    .Select(MapEntry)
                    .OrderBy(static entry => SortBucket(entry.Type))
                    .ThenBy(static entry => entry.Name, StringComparer.OrdinalIgnoreCase)
                    .ThenBy(static entry => entry.Name, StringComparer.Ordinal)
                    .ToArray(),
                response.IsTruncated);
        }
    }

    public async Task<Result<VolumeDownloadStream>> OpenDownloadAsync(
        DownloadVolumePathCommand command,
        CancellationToken cancellationToken)
    {
        if (!IsSupportedConnector(command.ConnectorType))
        {
            return Result.Failure<VolumeDownloadStream>(
                new BadRequestError("Volume content download is not supported by this platform connector."));
        }

        if (command.Path.ApiPath == "/")
            return Result.Failure<VolumeDownloadStream>(new BadRequestError("Downloading the volume root is not supported."));

        var helper = await CreateAndStartHelperAsync(command, cancellationToken);
        if (!helper.IsSuccess(out var session, out var createError))
        {
            return Result.Failure<VolumeDownloadStream>(createError);
        }

        ContainerBinaryExecResult? binaryExec = null;
        var transferredOwnership = false;

        try
        {
            var inspectArgs = BuildHelperCommand(
                "volume-helper",
                "inspect",
                "--root",
                HelperRoot,
                "--path",
                command.Path.ApiPath);

            var inspectResult = await RunJsonHelperAsync<VolumeHelperInspectResponse>(
                session,
                inspectArgs,
                cancellationToken);

            if (!inspectResult.IsSuccess(out var inspection, out var inspectError))
                return Result.Failure<VolumeDownloadStream>(inspectError);

            if (!string.IsNullOrWhiteSpace(inspection.ErrorCode))
                return Result.Failure<VolumeDownloadStream>(MapHelperError(inspection.ErrorCode, inspection.ErrorMessage));

            var entryType = MapEntryType(inspection.Type);
            if (!inspection.Exists)
                return Result.Failure<VolumeDownloadStream>(new NotFoundError("Volume path does not exist."));

            if (entryType == VolumeFileEntryType.Symlink)
                return Result.Failure<VolumeDownloadStream>(new BadRequestError("Volume path is a symlink and cannot be downloaded."));

            if (entryType is not (VolumeFileEntryType.File or VolumeFileEntryType.Directory))
                return Result.Failure<VolumeDownloadStream>(new BadRequestError("Volume path type is not downloadable."));

            var streamArgs = entryType == VolumeFileEntryType.Directory
                ? BuildHelperCommand(
                    "volume-helper",
                    "stream-directory",
                    "--root",
                    HelperRoot,
                    "--path",
                    command.Path.ApiPath)
                : BuildHelperCommand(
                    "volume-helper",
                    "stream-file",
                    "--root",
                    HelperRoot,
                    "--path",
                    command.Path.ApiPath);

            var exec = await session.ContainerConnector.ExecBinaryAsync(
                command.PlatformAddress,
                new ContainerBinaryExecRequest(session.ContainerId, streamArgs),
                cancellationToken);

            if (!exec.IsSuccess(out binaryExec, out var execError))
                return Result.Failure<VolumeDownloadStream>(execError);

            var disposed = false;
            async ValueTask CleanupAsync()
            {
                if (disposed)
                    return;

                disposed = true;
                await DisposeDownloadSessionAsync(binaryExec, session);
            }

            transferredOwnership = true;
            return new VolumeDownloadStream
            {
                PlatformId = command.PlatformId,
                VolumeName = command.VolumeName,
                Path = command.Path.ApiPath,
                EntryType = entryType,
                FileName = entryType == VolumeFileEntryType.Directory
                    ? $"{inspection.SafeFileName}.tar"
                    : inspection.SafeFileName,
                ContentType = entryType == VolumeFileEntryType.Directory
                    ? "application/x-tar"
                    : "application/octet-stream",
                ContentLength = entryType == VolumeFileEntryType.File ? inspection.Size : null,
                Chunks = StreamStdoutAndValidateAsync(binaryExec, cancellationToken),
                CleanupAsync = CleanupAsync
            };
        }
        finally
        {
            if (!transferredOwnership)
            {
                await DisposeDownloadSessionAsync(binaryExec, session);
            }
        }
    }

    private async Task<Result<HelperContainerSession>> CreateAndStartHelperAsync(
        ListVolumeDirectoryCommand command,
        CancellationToken cancellationToken)
        => await CreateAndStartHelperAsync(
            command.PlatformAddress,
            command.PlatformId,
            command.ConnectorType,
            command.VolumeName,
            cancellationToken);

    private async Task<Result<HelperContainerSession>> CreateAndStartHelperAsync(
        DownloadVolumePathCommand command,
        CancellationToken cancellationToken)
        => await CreateAndStartHelperAsync(
            command.PlatformAddress,
            command.PlatformId,
            command.ConnectorType,
            command.VolumeName,
            cancellationToken);

    private async Task<Result<HelperContainerSession>> CreateAndStartHelperAsync(
        string platformAddress,
        Guid platformId,
        PlatformConnectorType connectorType,
        string volumeName,
        CancellationToken cancellationToken)
    {
        var containerConnector = containerConnectorFactory.GetConnector(connectorType);
        var imageConnector = imageConnectorFactory.GetConnector(connectorType);
        var helperPlan = await ResolveHelperContainerPlanAsync(
            containerConnector,
            platformAddress,
            platformId,
            connectorType,
            volumeName,
            cancellationToken);
        var helperImage = helperPlan.Image;
        var containerName = $"citadel-volume-helper-{Guid.CreateVersion7():N}";

        var create = await TryCreateHelperAsync(
            containerConnector,
            platformAddress,
            platformId,
            helperPlan,
            containerName,
            cancellationToken);

        if (!create.IsSuccess(out var containerId, out var createError))
        {
            if (!IsMissingHelperImageError(createError))
                return Result.Failure<HelperContainerSession>(createError);

            if (!CanPullHelperImage(helperImage))
            {
                return Result.Failure<HelperContainerSession>(
                    new BadGatewayError($"Citadel volume helper image '{helperImage}' is not available on the target platform. Build or load the configured helper image on that Docker daemon before browsing volume contents."));
            }

            var pull = await PullHelperImageAsync(imageConnector, platformAddress, helperImage, cancellationToken);
            if (pull.IsFailure(out var pullError))
            {
                return Result.Failure<HelperContainerSession>(
                    new BadGatewayError($"Citadel volume helper image '{helperImage}' is not available on the target platform and could not be pulled: {pullError.Message}"));
            }

            create = await TryCreateHelperAsync(
                containerConnector,
                platformAddress,
                platformId,
                helperPlan,
                containerName,
                cancellationToken);

            if (!create.IsSuccess(out containerId, out createError))
            {
                return Result.Failure<HelperContainerSession>(
                    IsMissingHelperImageError(createError)
                        ? new BadGatewayError($"Citadel volume helper image '{helperImage}' is not available on the target platform.")
                        : createError);
            }
        }

        return await StartHelperAsync(containerConnector, platformAddress, containerId, cancellationToken);
    }

    private async Task<VolumeHelperContainerPlan> ResolveHelperContainerPlanAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        Guid platformId,
        PlatformConnectorType connectorType,
        string volumeName,
        CancellationToken cancellationToken)
    {
        var configuredImage = helperImageResolver.Resolve(connectorType);
        if (helperImageResolver.IsExplicitlyConfigured)
            return VolumeHelperContainerPlan.Create(configuredImage, volumeName);

        if (connectorType is PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent)
        {
            var runtimeImage = await agentRuntimeImageResolver.TryResolveAsync(
                containerConnector,
                platformAddress,
                platformId,
                connectorType,
                cancellationToken);

            return VolumeHelperContainerPlan.Create(runtimeImage ?? configuredImage, volumeName);
        }

        if (connectorType != PlatformConnectorType.Local)
            return VolumeHelperContainerPlan.Create(configuredImage, volumeName);

        var currentContainerId = Environment.MachineName;
        if (string.IsNullOrWhiteSpace(currentContainerId))
            return VolumeHelperContainerPlan.Create(configuredImage, volumeName);

        var currentContainer = await containerConnector.InspectAsync(
            new InspectContainerCommand(platformAddress, currentContainerId),
            cancellationToken);

        if (!currentContainer.IsSuccess(out var container))
            return VolumeHelperContainerPlan.Create(configuredImage, volumeName);

        var image = container.Config?.Image;
        if (string.IsNullOrWhiteSpace(image))
            return VolumeHelperContainerPlan.Create(configuredImage, volumeName);

        var sourceMount = container.Mounts.FirstOrDefault(static mount =>
            string.Equals(mount.Type, "bind", StringComparison.OrdinalIgnoreCase)
            && string.Equals(mount.Destination, "/src", StringComparison.Ordinal)
            && !string.IsNullOrWhiteSpace(mount.Source));

        return sourceMount?.Source is not null
            ? VolumeHelperContainerPlan.CreateWithSourceMount(image, volumeName, sourceMount.Source)
            : VolumeHelperContainerPlan.Create(image, volumeName);
    }

    private static async Task<Result<string>> TryCreateHelperAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        Guid platformId,
        VolumeHelperContainerPlan helperPlan,
        string containerName,
        CancellationToken cancellationToken)
        => await containerConnector.CreateAsync(
            new CreateContainerCommand(
                PlatformAddress: platformAddress,
                ImageId: helperPlan.Image,
                Name: containerName,
                WorkingDir: "/app",
                User: "0",
                MemoryLimit: HelperMemoryBytes,
                CpuQuota: null,
                MemoryReservation: null,
                MemorySwap: HelperMemoryBytes,
                PidsLimit: 64,
                AutoRemove: false,
                Privileged: false,
                ReadonlyRootfs: helperPlan.ReadonlyRootfs,
                RestartPolicy: null,
                Labels: new Dictionary<string, string>
                {
                    ["citadel.volume-browser"] = "true",
                    ["citadel.platform-id"] = platformId.ToString(),
                    ["citadel.volume-name"] = helperPlan.VolumeName
                },
                EnvVars: null,
                Ports: null,
                Volumes: null,
                Mounts: helperPlan.Mounts,
                CapAdd: ["DAC_READ_SEARCH"],
                CapDrop: ["ALL"],
                SecurityOpt: ["no-new-privileges"],
                NetworkMode: "none",
                Networks: null,
                EntryPoint: [HelperLauncherExecutable],
                Command: BuildHelperCommandArguments("volume-helper", "idle")),
            cancellationToken);

    private async Task<Result<HelperContainerSession>> StartHelperAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        var started = await containerConnector.PatchAsync(
            new PatchContainerCommand(ContainerAction.START, platformAddress, [containerId]),
            cancellationToken);

        if (started.IsFailure(out var startError))
        {
            var runtimeError = await GetHelperStartFailureAsync(
                containerConnector,
                platformAddress,
                containerId,
                CancellationToken.None);

            await DeleteHelperAsync(containerConnector, platformAddress, containerId, CancellationToken.None);
            return Result.Failure<HelperContainerSession>(
                new BadGatewayError(BuildHelperStartFailureMessage(startError.Message, runtimeError)));
        }

        var running = await EnsureHelperIsRunningAsync(
            containerConnector,
            platformAddress,
            containerId,
            cancellationToken);

        if (running.IsFailure(out var runningError))
        {
            await DeleteHelperAsync(containerConnector, platformAddress, containerId, CancellationToken.None);
            return Result.Failure<HelperContainerSession>(runningError);
        }

        return new HelperContainerSession(containerConnector, platformAddress, containerId, logger);
    }

    private static string[] BuildHelperCommand(params string[] args)
        => [HelperLauncherExecutable, .. BuildHelperCommandArguments(args)];

    private static List<string> BuildHelperCommandArguments(params string[] args)
        =>
        [
            "-c",
            HelperLauncherScript.ReplaceLineEndings("\n"),
            HelperLauncherName,
            .. args
        ];

    private static async Task<Result> PullHelperImageAsync(
        IImageConnector imageConnector,
        string platformAddress,
        string helperImage,
        CancellationToken cancellationToken)
    {
        var (fromImage, tag) = SplitImageReference(helperImage);
        await foreach (var item in imageConnector.PullImageProgressStreamAsync(
                           new PullImageCommand(
                               PlatformAddress: platformAddress,
                               FromImage: fromImage,
                               Tag: tag),
                           cancellationToken))
        {
            var errorMessage = item.ErrorMessage ?? item.Error?.Message;
            if (!string.IsNullOrWhiteSpace(errorMessage))
                return Result.Failure(new BadGatewayError(errorMessage));
        }

        return Result.Success();
    }

    private static async Task<Result<T>> RunJsonHelperAsync<T>(
        HelperContainerSession session,
        IReadOnlyList<string> args,
        CancellationToken cancellationToken)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(DefaultOperationTimeout);

        try
        {
            var exec = await session.ContainerConnector.ExecBinaryAsync(
                session.PlatformAddress,
                new ContainerBinaryExecRequest(session.ContainerId, args),
                timeout.Token);

            if (!exec.IsSuccess(out var binaryExec, out var execError))
                return Result.Failure<T>(execError);

            await using (binaryExec)
            {
                await using var stdout = new MemoryStream();
                await using var stderr = new MemoryStream();

                await foreach (var chunk in binaryExec.Output.WithCancellation(timeout.Token))
                {
                    var target = chunk.Stream == ContainerExecStream.Stderr ? stderr : stdout;
                    if (target.Length + chunk.Data.Length > MaxListingPayloadBytes)
                    {
                        return Result.Failure<T>(new BadRequestError("Volume helper output exceeded the allowed size."));
                    }

                    await target.WriteAsync(chunk.Data, timeout.Token);
                }

                var exitCode = await binaryExec.GetExitCodeAsync(timeout.Token);
                if (exitCode != 0)
                {
                    return Result.Failure<T>(new BadRequestError(GetErrorMessage(stderr, "Volume helper command failed.")));
                }

                stdout.Position = 0;
                object? result = typeof(T) == typeof(VolumeHelperListResponse)
                    ? await JsonSerializer.DeserializeAsync(
                        stdout,
                        VolumeHelperJsonContext.Default.VolumeHelperListResponse,
                        timeout.Token)
                    : await JsonSerializer.DeserializeAsync(
                        stdout,
                        VolumeHelperJsonContext.Default.VolumeHelperInspectResponse,
                        timeout.Token);

                return result is T typed
                    ? typed
                    : Result.Failure<T>(new BadGatewayError("Volume helper returned invalid output."));
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return Result.Failure<T>(new BadGatewayError("Volume helper command timed out."));
        }
        catch (JsonException)
        {
            return Result.Failure<T>(new BadGatewayError("Volume helper returned invalid JSON."));
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException)
        {
            return Result.Failure<T>(new BadGatewayError($"Volume helper stream failed: {ex.Message}"));
        }
    }

    private static async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamStdoutAndValidateAsync(
        ContainerBinaryExecResult binaryExec,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await using var stderr = new MemoryStream();

        await foreach (var chunk in binaryExec.Output.WithCancellation(cancellationToken))
        {
            if (chunk.Stream == ContainerExecStream.Stderr)
            {
                if (stderr.Length < 8192)
                    await stderr.WriteAsync(chunk.Data, cancellationToken);
                continue;
            }

            yield return chunk.Data;
        }

        var exitCode = await binaryExec.GetExitCodeAsync(cancellationToken);
        if (exitCode != 0)
            throw new IOException(GetErrorMessage(stderr, "Volume download failed."));
    }

    private static string GetErrorMessage(MemoryStream stderr, string fallback)
    {
        if (stderr.Length == 0)
            return fallback;

        return Encoding.UTF8.GetString(stderr.ToArray()).Trim() switch
        {
            { Length: > 0 } message => message,
            _ => fallback
        };
    }

    private static VolumeFileEntry MapEntry(VolumeHelperEntry entry)
        => new(
            entry.Name,
            entry.Path,
            MapEntryType(entry.Type),
            entry.Size,
            entry.ModifiedAt,
            entry.LinkTarget);

    private static VolumeFileEntryType MapEntryType(string? type)
        => type?.ToLowerInvariant() switch
        {
            "directory" => VolumeFileEntryType.Directory,
            "file" => VolumeFileEntryType.File,
            "symlink" => VolumeFileEntryType.Symlink,
            _ => VolumeFileEntryType.Other
        };

    private static int SortBucket(VolumeFileEntryType type)
        => type switch
        {
            VolumeFileEntryType.Directory => 0,
            VolumeFileEntryType.File => 1,
            VolumeFileEntryType.Symlink => 2,
            _ => 3
        };

    private static bool IsSupportedConnector(PlatformConnectorType connectorType)
        => connectorType is PlatformConnectorType.Local or PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent;

    private static Error MapHelperError(string code, string? message)
        => code switch
        {
            "VolumePathNotFound" => new NotFoundError(message ?? "Volume path does not exist."),
            "VolumePathInvalid" => new BadRequestError(message ?? "Volume path is invalid."),
            "VolumePathIsNotDirectory" => new BadRequestError(message ?? "Volume path is not a directory."),
            "VolumePathIsSymlink" => new BadRequestError(message ?? "Volume path is a symlink."),
            "VolumeDirectoryListingTooLarge" => new BadRequestError(message ?? "Volume directory listing is too large."),
            "VolumePathTypeUnsupported" => new BadRequestError(message ?? "Volume path type is not supported."),
            _ => new BadRequestError(message ?? "Volume helper command failed.")
        };

    private static bool IsMissingHelperImageError(IError error)
    {
        var message = error.Message;
        return message.Contains("No such image", StringComparison.OrdinalIgnoreCase)
               || message.Contains("image not found", StringComparison.OrdinalIgnoreCase)
               || message.Contains("not found: manifest", StringComparison.OrdinalIgnoreCase);
    }

    internal static bool CanPullHelperImage(string helperImage)
    {
        if (string.Equals(helperImage, DevelopmentHelperImage, StringComparison.OrdinalIgnoreCase))
            return false;

        var (_, tag) = SplitImageReference(helperImage);
        if (!string.Equals(tag, "dev", StringComparison.OrdinalIgnoreCase))
            return true;

        var slashIndex = helperImage.IndexOf('/');
        if (slashIndex < 0)
            return false;

        var registry = helperImage[..slashIndex];
        return registry.Contains('.', StringComparison.Ordinal) ||
               registry.Contains(':', StringComparison.Ordinal) ||
               string.Equals(registry, "localhost", StringComparison.OrdinalIgnoreCase);
    }

    private static (string FromImage, string? Tag) SplitImageReference(string image)
    {
        var slashIndex = image.LastIndexOf('/');
        var colonIndex = image.LastIndexOf(':');
        if (colonIndex > slashIndex)
            return (image[..colonIndex], image[(colonIndex + 1)..]);

        return (image, null);
    }

    private static async Task DeleteHelperAsync(
        IContainerConnector connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        await connector.DeleteAsync(
            new DeleteContainerCommand([containerId], platformAddress, Volume: false, Force: true, Link: false),
            cancellationToken);
    }

    private static async Task<string?> GetHelperStartFailureAsync(
        IContainerConnector connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        var inspect = await connector.InspectAsync(
            new InspectContainerCommand(platformAddress, containerId),
            cancellationToken);

        if (!inspect.IsSuccess(out var container))
            return null;

        var runtimeError = container.State?.Error;
        return string.IsNullOrWhiteSpace(runtimeError) ? null : runtimeError.Trim();
    }

    private static async Task<Result> EnsureHelperIsRunningAsync(
        IContainerConnector connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        await Task.Delay(HelperStartProbeDelay, cancellationToken);

        var inspect = await connector.InspectAsync(
            new InspectContainerCommand(platformAddress, containerId),
            cancellationToken);

        if (!inspect.IsSuccess(out var container, out var inspectError))
            return Result.Failure(inspectError);

        if (container.State?.Running == true)
            return Result.Success();

        var logs = await ReadHelperLogsAsync(connector, platformAddress, containerId, cancellationToken);
        return Result.Failure(new BadGatewayError(BuildHelperExitedMessage(container.State, logs)));
    }

    private static async Task<string?> ReadHelperLogsAsync(
        IContainerConnector connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(HelperLogTimeout);

        try
        {
            using var buffer = new MemoryStream();
            await foreach (var chunk in connector.StreamLogsAsync(
                               new StreamContainerLogsCommand(platformAddress, containerId),
                               timeout.Token))
            {
                var remaining = MaxHelperLogBytes - (int)buffer.Length;
                if (remaining <= 0)
                    break;

                await buffer.WriteAsync(chunk[..Math.Min(chunk.Length, remaining)], timeout.Token);
            }

            if (buffer.Length == 0)
                return null;

            return Encoding.UTF8.GetString(buffer.ToArray()).Trim();
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            return null;
        }
        catch (Exception) when (!cancellationToken.IsCancellationRequested)
        {
            return null;
        }
    }

    private static string BuildHelperExitedMessage(ContainerRuntimeState? state, string? logs)
    {
        var message = new StringBuilder("Volume helper container exited before it was ready.");

        if (state?.ExitCode is int exitCode)
            message.Append(" Exit code: ").Append(exitCode).Append('.');

        if (!string.IsNullOrWhiteSpace(state?.Error))
            message.Append(" Runtime error: ").Append(state.Error.Trim()).Append('.');

        if (!string.IsNullOrWhiteSpace(logs))
            message.Append(" Logs: ").Append(logs.Trim());

        return message.ToString();
    }

    private static string BuildHelperStartFailureMessage(string startError, string? runtimeError)
    {
        if (string.IsNullOrWhiteSpace(runtimeError))
            return $"Volume helper container failed to start: {startError}";

        return $"Volume helper container failed to start: {startError}. Docker runtime error: {runtimeError}";
    }

    private static async ValueTask DisposeDownloadSessionAsync(
        ContainerBinaryExecResult? binaryExec,
        HelperContainerSession session)
    {
        try
        {
            if (binaryExec is not null)
                await binaryExec.DisposeAsync();
        }
        finally
        {
            await session.DisposeAsync();
        }
    }

    private sealed class HelperContainerSession(
        IContainerConnector containerConnector,
        string platformAddress,
        string containerId,
        ILogger logger)
        : IAsyncDisposable
    {
        private bool disposed;

        public IContainerConnector ContainerConnector { get; } = containerConnector;
        public string PlatformAddress { get; } = platformAddress;
        public string ContainerId { get; } = containerId;

        public async ValueTask DisposeAsync()
        {
            if (disposed)
                return;

            disposed = true;
            try
            {
                await DeleteHelperAsync(ContainerConnector, PlatformAddress, ContainerId, CancellationToken.None);
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Failed to delete volume helper container {ContainerId}", ContainerId);
            }
        }
    }

    internal sealed record VolumeHelperListResponse(
        string Path,
        VolumeHelperEntry[] Entries,
        bool IsTruncated,
        string? ErrorCode = null,
        string? ErrorMessage = null);

    internal sealed record VolumeHelperInspectResponse(
        bool Exists,
        string Type,
        long? Size,
        string SafeFileName,
        string? ErrorCode = null,
        string? ErrorMessage = null);

    internal sealed record VolumeHelperEntry(
        string Name,
        string Path,
        string Type,
        long? Size,
        DateTimeOffset? ModifiedAt,
        string? LinkTarget);

    private sealed record VolumeHelperContainerPlan(
        string Image,
        string VolumeName,
        List<HostMount> Mounts,
        bool ReadonlyRootfs)
    {
        public static VolumeHelperContainerPlan Create(string image, string volumeName)
            => new(image, volumeName, [CreateVolumeMount(volumeName)], ReadonlyRootfs: true);

        public static VolumeHelperContainerPlan CreateWithSourceMount(
            string image,
            string volumeName,
            string sourceRoot)
            =>
            new(
                image,
                volumeName,
                [
                    CreateVolumeMount(volumeName),
                    new HostMount(
                        Target: "/src",
                        Source: sourceRoot,
                        Type: "bind",
                        ReadOnly: true,
                        Consistency: null,
                        BindOptions: null,
                        VolumeOptions: null)
                ],
                ReadonlyRootfs: true);

        private static HostMount CreateVolumeMount(string volumeName)
            => new(
                Target: HelperRoot,
                Source: volumeName,
                Type: "volume",
                ReadOnly: true,
                Consistency: null,
                BindOptions: null,
                VolumeOptions: null);
    }
}

internal sealed class VolumeHelperImageResolver(
    IConfiguration configuration,
    IOptions<EdgeAgentOptions> edgeAgentOptions) : IVolumeHelperImageResolver
{
    private const string DefaultCoreImageRepository = "ghcr.io/citadel-p/citadel";

    public bool IsExplicitlyConfigured => !string.IsNullOrWhiteSpace(configuration["VolumeBrowser:HelperImage"]);

    public string Resolve(PlatformConnectorType connectorType)
    {
        var explicitImage = configuration["VolumeBrowser:HelperImage"];
        if (!string.IsNullOrWhiteSpace(explicitImage))
            return explicitImage.Trim();

        if (connectorType is PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent)
            return edgeAgentOptions.Value.GetAgentImage();

        return $"{DefaultCoreImageRepository}:{NormalizeDockerTag(ApplicationVersion.CoreVersion)}";
    }

    private static string NormalizeDockerTag(string tag)
    {
        var normalized = tag.Trim();
        var metadataIndex = normalized.IndexOf('+', StringComparison.Ordinal);
        if (metadataIndex >= 0)
            normalized = normalized[..metadataIndex];

        if (normalized.Length > 1 && normalized[0] is 'v' or 'V' && char.IsDigit(normalized[1]))
            normalized = normalized[1..];

        Span<char> chars = normalized.Length <= 256
            ? stackalloc char[normalized.Length]
            : new char[normalized.Length];

        var length = 0;
        foreach (var ch in normalized)
            chars[length++] = IsDockerTagChar(ch) ? ch : '-';

        normalized = new string(chars[..length]).Trim('.', '-');
        return string.IsNullOrWhiteSpace(normalized) ? "latest" : normalized;
    }

    private static bool IsDockerTagChar(char ch) =>
        (ch >= 'a' && ch <= 'z') ||
        (ch >= 'A' && ch <= 'Z') ||
        (ch >= '0' && ch <= '9') ||
        ch is '_' or '.' or '-';
}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default, PropertyNameCaseInsensitive = true)]
[JsonSerializable(typeof(VolumeContentService.VolumeHelperListResponse))]
[JsonSerializable(typeof(VolumeContentService.VolumeHelperInspectResponse))]
internal partial class VolumeHelperJsonContext : JsonSerializerContext
{
}
