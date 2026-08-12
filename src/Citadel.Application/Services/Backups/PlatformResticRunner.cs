using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Entities.Platforms;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.RegularExpressions;

namespace Application.Services.Backups;

internal interface IPlatformResticRunner
{
    IAsyncEnumerable<ResticProcessEvent> RunAsync(PlatformResticCommand command, CancellationToken cancellationToken);
}

internal sealed record PlatformResticCommand(
    Guid PlatformId,
    string PlatformAddress,
    PlatformConnectorType ConnectorType,
    string ResticExecutable,
    IReadOnlyList<string> Arguments,
    IReadOnlyDictionary<string, string> Environment,
    TimeSpan Timeout,
    IReadOnlyCollection<string> RedactionValues,
    int MaxLineBytes,
    string? SourceVolumeName,
    string? TargetVolumeName,
    string? RepositoryHostPath,
    string? NetworkMode,
    string? DockerNodeId = null);

internal sealed partial class PlatformResticRunner(
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IVolumeHelperImageResolver helperImageResolver,
    IAgentRuntimeImageResolver agentRuntimeImageResolver,
    ISwarmNodeRuntimeConnector swarmNodeRuntimeConnector,
    IServiceScopeFactory scopeFactory)
    : IPlatformResticRunner
{
    private const string HelperExecutable = "/bin/sh";
    private const string HelperSourceRoot = "/source";
    private const string HelperTargetRoot = "/target";
    private const string HelperRepositoryRoot = "/repository";
    private const string HelperWorkDir = "/tmp";
    private const long HelperMemoryBytes = 512L * 1024 * 1024;
    private static readonly TimeSpan HelperStartProbeDelay = TimeSpan.FromMilliseconds(150);

    public async IAsyncEnumerable<ResticProcessEvent> RunAsync(
        PlatformResticCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var ct = cancellationToken;

        var containerConnector = containerConnectorFactory.GetConnector(command.ConnectorType);
        var imageConnector = imageConnectorFactory.GetConnector(command.ConnectorType);
        Platform? platform = null;
        string? swarmNodeHelperImage = null;
        if (!string.IsNullOrWhiteSpace(command.DockerNodeId))
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            platform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, ct);
            if (platform?.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            {
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, "The selected backup Node does not belong to a Docker Swarm Platform.");
                yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
                yield break;
            }

            var installation = await unitOfWork.EdgeAgents.GetNodeAgentInstallationAsync(command.PlatformId, ct);
            if (!string.IsNullOrWhiteSpace(installation?.DockerServiceId))
            {
                var service = await unitOfWork.Swarm.GetServiceAsync(
                    command.PlatformId,
                    installation.DockerServiceId,
                    ct);
                if (service is not null
                    && SwarmNodeAgentInfrastructure.HasOwnership(service, platform)
                    && !string.IsNullOrWhiteSpace(service.Image))
                {
                    swarmNodeHelperImage = service.Image;
                }
            }
        }

        var targetsWorker = platform?.PlatformDescriptor is DockerSwarmPlatformDescriptor descriptor
            && !string.Equals(descriptor.NodeID, command.DockerNodeId, StringComparison.Ordinal);
        IBackupContainerRuntime runtime = targetsWorker
            ? new SwarmNodeBackupContainerRuntime(swarmNodeRuntimeConnector, platform!, command.DockerNodeId!)
            : new ConnectorBackupContainerRuntime(containerConnector);
        var helperImage = targetsWorker
            ? swarmNodeHelperImage ?? helperImageResolver.Resolve(PlatformConnectorType.Agent)
            : await ResolveHelperImageAsync(containerConnector, command, ct);

        if (targetsWorker && !string.IsNullOrWhiteSpace(command.RepositoryHostPath))
        {
            yield return new ResticProcessEvent(ResticProcessStream.StdErr, "Filesystem backup repositories are not supported for Swarm Node backups.");
            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
            yield break;
        }

        var preparedRepositoryPath = await EnsureRepositoryHostPathAsync(containerConnector, imageConnector, command, helperImage, ct);
        if (preparedRepositoryPath.IsFailure(out var prepareError))
        {
            yield return new ResticProcessEvent(
                ResticProcessStream.StdErr,
                Sanitize(prepareError.Message, command));
            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
            yield break;
        }

        var helper = await CreateAndStartHelperAsync(runtime, imageConnector, command, helperImage, !targetsWorker, ct);
        if (!helper.IsSuccess(out var containerId, out var helperError))
        {
            yield return new ResticProcessEvent(
                ResticProcessStream.StdErr,
                Sanitize(helperError?.Message ?? "Backup helper container could not be started.", command));
            yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
            yield break;
        }

        try
        {
            var exec = await runtime.ExecBinaryAsync(
                command.PlatformAddress,
                new ContainerBinaryExecRequest(
                    containerId,
                    [command.ResticExecutable, .. command.Arguments],
                    command.Environment,
                    AttachStdout: true,
                    AttachStderr: true,
                    Tty: false),
                ct);

            if (!exec.IsSuccess(out var binaryExec, out var execError))
            {
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, Sanitize(ToExecErrorMessage(execError.Message, command, helperImage), command));
                yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
                yield break;
            }

            await using (binaryExec)
            {
                var stdout = new StreamLineBuffer(ResticProcessStream.StdOut);
                var stderr = new StreamLineBuffer(ResticProcessStream.StdErr);

                await foreach (var chunk in binaryExec.Output.WithCancellation(ct))
                {
                    var buffer = chunk.Stream == ContainerExecStream.Stderr ? stderr : stdout;
                    foreach (var line in buffer.Append(chunk.Data))
                        yield return new ResticProcessEvent(buffer.Stream, Sanitize(line, command));
                }

                foreach (var line in stdout.Flush())
                    yield return new ResticProcessEvent(ResticProcessStream.StdOut, Sanitize(line, command));
                foreach (var line in stderr.Flush())
                    yield return new ResticProcessEvent(ResticProcessStream.StdErr, Sanitize(line, command));

                var exitCode = await binaryExec.GetExitCodeAsync(CancellationToken.None);
                yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: exitCode ?? -1);
            }
        }
        finally
        {
            await runtime.DeleteAsync(
                new DeleteContainerCommand([containerId], command.PlatformAddress, Volume: false, Force: true, Link: false),
                CancellationToken.None);
        }
    }

    private async Task<string> ResolveHelperImageAsync(
        IContainerConnector containerConnector,
        PlatformResticCommand command,
        CancellationToken cancellationToken)
    {
        var configuredImage = helperImageResolver.Resolve(command.ConnectorType);
        if (helperImageResolver.IsExplicitlyConfigured)
            return configuredImage;

        if (command.ConnectorType is PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent)
        {
            return await agentRuntimeImageResolver.TryResolveAsync(
                containerConnector,
                command.PlatformAddress,
                command.PlatformId,
                command.ConnectorType,
                cancellationToken) ?? configuredImage;
        }

        if (command.ConnectorType != PlatformConnectorType.Local)
            return configuredImage;

        var currentContainerId = Environment.MachineName;
        if (string.IsNullOrWhiteSpace(currentContainerId))
            return configuredImage;

        var currentContainer = await containerConnector.InspectAsync(
            new InspectContainerCommand(command.PlatformAddress, currentContainerId),
            cancellationToken);

        return currentContainer.IsSuccess(out var container) && !string.IsNullOrWhiteSpace(container.Config?.Image)
            ? container.Config.Image
            : configuredImage;
    }

    private async Task<LightResults.Result<string>> CreateAndStartHelperAsync(
        IBackupContainerRuntime runtime,
        IImageConnector imageConnector,
        PlatformResticCommand command,
        string helperImage,
        bool canPullImage,
        CancellationToken cancellationToken)
    {
        var containerName = $"citadel-backup-helper-{Guid.CreateVersion7():N}";
        var create = await TryCreateHelperAsync(runtime, command, helperImage, containerName, cancellationToken);

        if (!create.IsSuccess(out var containerId, out var createError))
        {
            if (!IsMissingHelperImageError(createError))
                return LightResults.Result.Failure<string>(createError);

            if (!canPullImage || !CanPullHelperImage(helperImage))
            {
                return LightResults.Result.Failure<string>(
                    new Hosting.Common.ErrorTypes.BadGatewayError($"Backup helper image '{helperImage}' is not available on the target platform. Build or load the configured helper image on that Docker daemon before running platform backups."));
            }

            var pull = await PullHelperImageAsync(imageConnector, command.PlatformAddress, helperImage, cancellationToken);
            if (pull.IsFailure(out var pullError))
                return LightResults.Result.Failure<string>(pullError);

            create = await TryCreateHelperAsync(runtime, command, helperImage, containerName, cancellationToken);
            if (!create.IsSuccess(out containerId, out createError))
                return LightResults.Result.Failure<string>(createError);
        }

        var started = await runtime.PatchAsync(
            new PatchContainerCommand(ContainerAction.START, command.PlatformAddress, [containerId]),
            cancellationToken);

        if (started.IsFailure())
        {
            await DeleteHelperAsync(runtime, command.PlatformAddress, containerId, CancellationToken.None);
            return LightResults.Result.Failure<string>(
                new Hosting.Common.ErrorTypes.BadGatewayError("Backup helper container failed to start."));
        }

        var running = await EnsureHelperIsRunningAsync(runtime, command.PlatformAddress, containerId, cancellationToken);
        if (running.IsFailure(out var runningError))
        {
            await DeleteHelperAsync(runtime, command.PlatformAddress, containerId, CancellationToken.None);
            return LightResults.Result.Failure<string>(runningError);
        }

        return containerId;
    }

    private async Task<LightResults.Result> EnsureRepositoryHostPathAsync(
        IContainerConnector containerConnector,
        IImageConnector imageConnector,
        PlatformResticCommand command,
        string helperImage,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(command.RepositoryHostPath))
            return LightResults.Result.Success();

        var preparation = BuildRepositoryPathPreparation(command.RepositoryHostPath);
        if (!preparation.IsSuccess(out var path, out var pathError))
            return LightResults.Result.Failure(pathError);

        if (path is null)
            return LightResults.Result.Success();

        var containerName = $"citadel-backup-path-helper-{Guid.CreateVersion7():N}";
        var runtime = new ConnectorBackupContainerRuntime(containerConnector);
        var create = await TryCreateRepositoryPathHelperAsync(
            containerConnector,
            command,
            helperImage,
            containerName,
            path,
            cancellationToken);

        if (!create.IsSuccess(out var containerId, out var createError))
        {
            if (!IsMissingHelperImageError(createError))
                return LightResults.Result.Failure(createError);

            if (!CanPullHelperImage(helperImage))
            {
                return LightResults.Result.Failure(
                    new Hosting.Common.ErrorTypes.BadGatewayError($"Backup helper image '{helperImage}' is not available on the target platform. Build or load the configured helper image on that Docker daemon before running platform backups."));
            }

            var pull = await PullHelperImageAsync(imageConnector, command.PlatformAddress, helperImage, cancellationToken);
            if (pull.IsFailure(out var pullError))
                return LightResults.Result.Failure(pullError);

            create = await TryCreateRepositoryPathHelperAsync(
                containerConnector,
                command,
                helperImage,
                containerName,
                path,
                cancellationToken);
            if (!create.IsSuccess(out containerId, out createError))
                return LightResults.Result.Failure(createError);
        }

        try
        {
            var started = await containerConnector.PatchAsync(
                new PatchContainerCommand(ContainerAction.START, command.PlatformAddress, [containerId]),
                cancellationToken);
            if (started.IsFailure(out var startError))
                return LightResults.Result.Failure(startError);

            var running = await EnsureHelperIsRunningAsync(runtime, command.PlatformAddress, containerId, cancellationToken);
            if (running.IsFailure(out var runningError))
                return LightResults.Result.Failure(runningError);

            return await RunRepositoryPathPreparationAsync(containerConnector, command, containerId, path.ContainerPath, cancellationToken);
        }
        finally
        {
            await DeleteHelperAsync(runtime, command.PlatformAddress, containerId, CancellationToken.None);
        }
    }

    private static async Task<LightResults.Result<string>> TryCreateHelperAsync(
        IBackupContainerRuntime runtime,
        PlatformResticCommand command,
        string helperImage,
        string containerName,
        CancellationToken cancellationToken)
    {
        var mounts = new List<HostMount>();
        if (!string.IsNullOrWhiteSpace(command.SourceVolumeName))
        {
            mounts.Add(new HostMount(
                Target: HelperSourceRoot,
                Source: command.SourceVolumeName,
                Type: "volume",
                ReadOnly: true,
                Consistency: null,
                BindOptions: null,
                VolumeOptions: null));
        }

        if (!string.IsNullOrWhiteSpace(command.TargetVolumeName))
        {
            mounts.Add(new HostMount(
                Target: HelperTargetRoot,
                Source: command.TargetVolumeName,
                Type: "volume",
                ReadOnly: false,
                Consistency: null,
                BindOptions: null,
                VolumeOptions: null));
        }

        if (!string.IsNullOrWhiteSpace(command.RepositoryHostPath))
        {
            mounts.Add(new HostMount(
                Target: HelperRepositoryRoot,
                Source: command.RepositoryHostPath,
                Type: "bind",
                ReadOnly: false,
                Consistency: null,
                BindOptions: new BindOptions(
                    Propagation: null,
                    NonRecursive: false,
                    CreateMountpoint: true,
                    ReadOnlyNonRecursive: false,
                    ReadOnlyForceRecursive: false),
                VolumeOptions: null));
        }

        return await runtime.CreateAsync(
            new CreateContainerCommand(
                PlatformAddress: command.PlatformAddress,
                ImageId: helperImage,
                Name: containerName,
                WorkingDir: HelperWorkDir,
                User: "0",
                MemoryLimit: HelperMemoryBytes,
                CpuQuota: null,
                MemoryReservation: null,
                MemorySwap: HelperMemoryBytes,
                PidsLimit: 128,
                AutoRemove: true,
                Privileged: false,
                ReadonlyRootfs: false,
                RestartPolicy: null,
                Labels: new Dictionary<string, string>
                {
                    ["citadel.backup-helper"] = "true",
                    ["citadel.platform-id"] = command.PlatformId.ToString()
                },
                EnvVars: null,
                Ports: null,
                Volumes: null,
                Mounts: mounts,
                CapAdd: ["DAC_READ_SEARCH", "FOWNER"],
                CapDrop: ["ALL"],
                SecurityOpt: ["no-new-privileges"],
                NetworkMode: command.NetworkMode,
                Networks: null,
                EntryPoint: [HelperExecutable],
                Command:
                [
                    "-c",
                    $"trap 'exit 0' TERM INT; sleep {GetHelperLifetimeSeconds(command.Timeout)}"
                ]),
            cancellationToken);
    }

    private static async Task<LightResults.Result<string>> TryCreateRepositoryPathHelperAsync(
        IContainerConnector containerConnector,
        PlatformResticCommand command,
        string helperImage,
        string containerName,
        RepositoryPathPreparation path,
        CancellationToken cancellationToken)
    {
        var create = await containerConnector.CreateAsync(
            new CreateContainerCommand(
                PlatformAddress: command.PlatformAddress,
                ImageId: helperImage,
                Name: containerName,
                WorkingDir: HelperWorkDir,
                User: "0",
                MemoryLimit: HelperMemoryBytes,
                CpuQuota: null,
                MemoryReservation: null,
                MemorySwap: HelperMemoryBytes,
                PidsLimit: 128,
                AutoRemove: true,
                Privileged: false,
                ReadonlyRootfs: false,
                RestartPolicy: null,
                Labels: new Dictionary<string, string>
                {
                    ["citadel.backup-helper"] = "true",
                    ["citadel.backup-path-helper"] = "true",
                    ["citadel.platform-id"] = command.PlatformId.ToString()
                },
                EnvVars: null,
                Ports: null,
                Volumes: null,
                Mounts:
                [
                    new HostMount(
                        Target: path.ContainerParentPath,
                        Source: path.HostParentPath,
                        Type: "bind",
                        ReadOnly: false,
                        Consistency: null,
                        BindOptions: null,
                        VolumeOptions: null)
                ],
                CapAdd: ["DAC_READ_SEARCH", "FOWNER"],
                CapDrop: ["ALL"],
                SecurityOpt: ["no-new-privileges"],
                NetworkMode: "none",
                Networks: null,
                EntryPoint: [HelperExecutable],
                Command:
                [
                    "-c",
                    $"trap 'exit 0' TERM INT; sleep {GetHelperLifetimeSeconds(command.Timeout)}"
                ]),
            cancellationToken);

        if (create.IsSuccess() || create.Errors.All(error => !IsMissingHostPathError(error.Message)))
            return create;

        return LightResults.Result.Failure<string>(
            new Hosting.Common.ErrorTypes.BadGatewayError($"Backup repository parent path '{path.HostParentPath}' does not exist on the target platform. Create that parent directory first or choose a repository path under an existing host directory."));
    }

    private static async Task<LightResults.Result> RunRepositoryPathPreparationAsync(
        IContainerConnector containerConnector,
        PlatformResticCommand command,
        string containerId,
        string containerPath,
        CancellationToken cancellationToken)
    {
        var exec = await containerConnector.ExecBinaryAsync(
            command.PlatformAddress,
            new ContainerBinaryExecRequest(
                containerId,
                [HelperExecutable, "-c", $"mkdir -p -- {ShellQuote(containerPath)}"],
                Environment: null,
                AttachStdout: true,
                AttachStderr: true,
                Tty: false),
            cancellationToken);

        if (!exec.IsSuccess(out var binaryExec, out var execError))
            return LightResults.Result.Failure(execError);

        await using (binaryExec)
        {
            var stdout = new StreamLineBuffer(ResticProcessStream.StdOut);
            var stderr = new StreamLineBuffer(ResticProcessStream.StdErr);
            var output = new List<string>();

            await foreach (var chunk in binaryExec.Output.WithCancellation(cancellationToken))
            {
                var buffer = chunk.Stream == ContainerExecStream.Stderr ? stderr : stdout;
                foreach (var line in buffer.Append(chunk.Data))
                    output.Add(Sanitize(line, command));
            }

            foreach (var line in stdout.Flush())
                output.Add(Sanitize(line, command));
            foreach (var line in stderr.Flush())
                output.Add(Sanitize(line, command));

            var exitCode = await binaryExec.GetExitCodeAsync(CancellationToken.None);
            if (exitCode is 0)
                return LightResults.Result.Success();

            var message = output.Count > 0
                ? string.Join('\n', output)
                : $"Backup repository path could not be created. Helper exited with code {exitCode ?? -1}.";
            return LightResults.Result.Failure(
                new Hosting.Common.ErrorTypes.BadGatewayError(message));
        }
    }

    private static async Task<LightResults.Result> EnsureHelperIsRunningAsync(
        IBackupContainerRuntime containerConnector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        await Task.Delay(HelperStartProbeDelay, cancellationToken);
        var inspect = await containerConnector.InspectAsync(
            new InspectContainerCommand(platformAddress, containerId),
            cancellationToken);

        if (!inspect.IsSuccess(out var info, out var error))
            return LightResults.Result.Failure(error);

        if (info.State?.Running == true)
            return LightResults.Result.Success();

        var message = info.State?.Error;
        return LightResults.Result.Failure(
            new Hosting.Common.ErrorTypes.BadGatewayError(string.IsNullOrWhiteSpace(message)
                ? "Backup helper container exited before it was ready."
                : $"Backup helper container exited before it was ready: {message}"));
    }

    private static async Task<LightResults.Result> PullHelperImageAsync(
        IImageConnector imageConnector,
        string platformAddress,
        string helperImage,
        CancellationToken cancellationToken)
    {
        var (fromImage, tag) = SplitImageReference(helperImage);
        await foreach (var item in imageConnector.PullImageProgressStreamAsync(
                           new PullImageCommand(platformAddress, fromImage, tag),
                           cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(item.ErrorMessage) || !string.IsNullOrWhiteSpace(item.Error?.Message))
            {
                return LightResults.Result.Failure(
                    new Hosting.Common.ErrorTypes.BadGatewayError($"Backup helper image '{helperImage}' could not be pulled: {item.ErrorMessage ?? item.Error?.Message}"));
            }
        }

        return LightResults.Result.Success();
    }

    private static async Task DeleteHelperAsync(
        IBackupContainerRuntime connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        await connector.DeleteAsync(
            new DeleteContainerCommand([containerId], platformAddress, Volume: false, Force: true, Link: false),
            cancellationToken);
    }

    private interface IBackupContainerRuntime
    {
        Task<LightResults.Result<string>> CreateAsync(
            CreateContainerCommand command,
            CancellationToken cancellationToken);

        Task<LightResults.Result> PatchAsync(
            PatchContainerCommand command,
            CancellationToken cancellationToken);

        Task<LightResults.Result> DeleteAsync(
            DeleteContainerCommand command,
            CancellationToken cancellationToken);

        Task<LightResults.Result<ContainerInspectionInfo>> InspectAsync(
            InspectContainerCommand command,
            CancellationToken cancellationToken);

        Task<LightResults.Result<ContainerBinaryExecResult>> ExecBinaryAsync(
            string platformAddress,
            ContainerBinaryExecRequest request,
            CancellationToken cancellationToken);
    }

    private sealed class ConnectorBackupContainerRuntime(IContainerConnector connector) : IBackupContainerRuntime
    {
        public Task<LightResults.Result<string>> CreateAsync(CreateContainerCommand command, CancellationToken cancellationToken)
            => connector.CreateAsync(command, cancellationToken);

        public Task<LightResults.Result> PatchAsync(PatchContainerCommand command, CancellationToken cancellationToken)
            => connector.PatchAsync(command, cancellationToken);

        public Task<LightResults.Result> DeleteAsync(DeleteContainerCommand command, CancellationToken cancellationToken)
            => connector.DeleteAsync(command, cancellationToken);

        public Task<LightResults.Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand command, CancellationToken cancellationToken)
            => connector.InspectAsync(command, cancellationToken);

        public Task<LightResults.Result<ContainerBinaryExecResult>> ExecBinaryAsync(
            string platformAddress,
            ContainerBinaryExecRequest request,
            CancellationToken cancellationToken)
            => connector.ExecBinaryAsync(platformAddress, request, cancellationToken);
    }

    private sealed class SwarmNodeBackupContainerRuntime(
        ISwarmNodeRuntimeConnector connector,
        Platform platform,
        string dockerNodeId) : IBackupContainerRuntime
    {
        public Task<LightResults.Result<string>> CreateAsync(CreateContainerCommand command, CancellationToken cancellationToken)
            => connector.CreateContainerAsync(platform, dockerNodeId, command, cancellationToken);

        public Task<LightResults.Result> PatchAsync(PatchContainerCommand command, CancellationToken cancellationToken)
            => connector.PatchContainersAsync(platform, dockerNodeId, command.Action, [.. command.ContainerIds], cancellationToken);

        public Task<LightResults.Result> DeleteAsync(DeleteContainerCommand command, CancellationToken cancellationToken)
            => connector.DeleteContainersAsync(
                platform,
                dockerNodeId,
                [.. command.ContainerIds],
                command.Volume ?? false,
                command.Force ?? false,
                command.Link ?? false,
                cancellationToken);

        public Task<LightResults.Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand command, CancellationToken cancellationToken)
            => connector.InspectContainerAsync(platform, dockerNodeId, command.ContainerId, cancellationToken);

        public Task<LightResults.Result<ContainerBinaryExecResult>> ExecBinaryAsync(
            string platformAddress,
            ContainerBinaryExecRequest request,
            CancellationToken cancellationToken)
            => connector.ExecBinaryAsync(platform, dockerNodeId, request, cancellationToken);
    }

    private static bool IsMissingHelperImageError(LightResults.IError error)
    {
        var message = error.Message;
        return message.Contains("No such image", StringComparison.OrdinalIgnoreCase)
               || message.Contains("image not found", StringComparison.OrdinalIgnoreCase)
               || message.Contains("not found: manifest", StringComparison.OrdinalIgnoreCase);
    }

    private static long GetHelperLifetimeSeconds(TimeSpan timeout)
        => Math.Max(300L, (long)Math.Ceiling(timeout.TotalSeconds) + 300L);

    private static bool IsMissingHostPathError(string message)
        => message.Contains("bind source path does not exist", StringComparison.OrdinalIgnoreCase)
           || message.Contains("invalid mount config for type \"bind\"", StringComparison.OrdinalIgnoreCase);

    private static string ToExecErrorMessage(string message, PlatformResticCommand command, string helperImage)
    {
        if (!IsMissingExecutableError(message, command.ResticExecutable))
            return message;

        return $"Backup helper image '{helperImage}' does not include '{command.ResticExecutable}'. Rebuild or pull the matching Citadel helper image before running platform backups.";
    }

    private static bool IsMissingExecutableError(string message, string executable)
        => message.Contains($"exec: \"{executable}\"", StringComparison.OrdinalIgnoreCase)
           && (message.Contains("executable file not found", StringComparison.OrdinalIgnoreCase)
               || message.Contains("not found in $PATH", StringComparison.OrdinalIgnoreCase));

    private static bool CanPullHelperImage(string helperImage)
    {
        if (string.Equals(helperImage, VolumeContentService.DevelopmentHelperImage, StringComparison.OrdinalIgnoreCase))
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

    private static string Sanitize(string value, PlatformResticCommand command)
    {
        var sanitized = AnsiRegex().Replace(value, string.Empty);
        foreach (var secret in command.RedactionValues
            .Where(static x => !string.IsNullOrEmpty(x))
            .Distinct(StringComparer.Ordinal)
            .OrderByDescending(static x => x.Length))
        {
            sanitized = sanitized.Replace(secret, "********", StringComparison.Ordinal);
        }

        return sanitized.Length <= command.MaxLineBytes
            ? sanitized
            : sanitized[..command.MaxLineBytes] + "...";
    }

    private static LightResults.Result<RepositoryPathPreparation?> BuildRepositoryPathPreparation(string repositoryHostPath)
    {
        var path = repositoryHostPath.Trim();
        if (string.IsNullOrWhiteSpace(path))
            return LightResults.Result.Success<RepositoryPathPreparation?>(null);

        if (!path.StartsWith("/", StringComparison.Ordinal))
            return LightResults.Result.Success<RepositoryPathPreparation?>(null);

        var trimmed = path.TrimEnd('/');
        if (trimmed.Length == 0)
            return LightResults.Result.Success<RepositoryPathPreparation?>(null);

        var separatorIndex = trimmed.LastIndexOf('/');
        var parent = separatorIndex <= 0 ? "/" : trimmed[..separatorIndex];
        var leaf = trimmed[(separatorIndex + 1)..];
        if (string.IsNullOrWhiteSpace(leaf))
            return LightResults.Result.Success<RepositoryPathPreparation?>(null);

        return new RepositoryPathPreparation(
            parent,
            "/host-parent",
            $"/host-parent/{leaf}");
    }

    private static string ShellQuote(string value)
        => "'" + value.Replace("'", "'\"'\"'", StringComparison.Ordinal) + "'";

    [GeneratedRegex(@"\x1B\[[0-?]*[ -/]*[@-~]", RegexOptions.Compiled)]
    private static partial Regex AnsiRegex();

    private sealed record RepositoryPathPreparation(
        string HostParentPath,
        string ContainerParentPath,
        string ContainerPath);

    private sealed class StreamLineBuffer(ResticProcessStream stream)
    {
        private readonly Decoder decoder = Encoding.UTF8.GetDecoder();
        private readonly StringBuilder pending = new();

        public ResticProcessStream Stream { get; } = stream;

        public IEnumerable<string> Append(ReadOnlyMemory<byte> data)
        {
            var chars = new char[Encoding.UTF8.GetMaxCharCount(data.Length)];
            decoder.Convert(data.Span, chars, flush: false, out _, out var charsUsed, out _);
            pending.Append(chars, 0, charsUsed);
            return Drain(complete: false);
        }

        public IEnumerable<string> Flush()
            => Drain(complete: true);

        private IEnumerable<string> Drain(bool complete)
        {
            var start = 0;
            for (var i = 0; i < pending.Length; i++)
            {
                if (pending[i] != '\n')
                    continue;

                var length = i - start;
                if (length > 0 && pending[start + length - 1] == '\r')
                    length--;

                yield return pending.ToString(start, length);
                start = i + 1;
            }

            if (start > 0)
                pending.Remove(0, start);

            if (complete && pending.Length > 0)
            {
                var line = pending.ToString();
                pending.Clear();
                yield return line;
            }
        }
    }
}
