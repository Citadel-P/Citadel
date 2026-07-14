using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
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
    string? RepositoryHostPath,
    string? NetworkMode);

internal sealed partial class PlatformResticRunner(
    IConnectorFactory<IContainerConnector> containerConnectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IVolumeHelperImageResolver helperImageResolver)
    : IPlatformResticRunner
{
    private const string HelperExecutable = "/bin/sh";
    private const string HelperSourceRoot = "/source";
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
        var helper = await CreateAndStartHelperAsync(containerConnector, imageConnector, command, ct);
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
            var exec = await containerConnector.ExecBinaryAsync(
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
                yield return new ResticProcessEvent(ResticProcessStream.StdErr, Sanitize(execError.Message, command));
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
            await containerConnector.DeleteAsync(
                new DeleteContainerCommand([containerId], command.PlatformAddress, Volume: false, Force: true, Link: false),
                CancellationToken.None);
        }
    }

    private async Task<LightResults.Result<string>> CreateAndStartHelperAsync(
        IContainerConnector containerConnector,
        IImageConnector imageConnector,
        PlatformResticCommand command,
        CancellationToken cancellationToken)
    {
        var helperImage = helperImageResolver.Resolve();
        var containerName = $"citadel-backup-helper-{Guid.CreateVersion7():N}";
        var create = await TryCreateHelperAsync(containerConnector, command, helperImage, containerName, cancellationToken);

        if (!create.IsSuccess(out var containerId, out var createError))
        {
            if (!IsMissingHelperImageError(createError))
                return LightResults.Result.Failure<string>(createError);

            if (!CanPullHelperImage(helperImage))
            {
                return LightResults.Result.Failure<string>(
                    new Hosting.Common.ErrorTypes.BadGatewayError($"Backup helper image '{helperImage}' is not available on the target platform. Build or load the configured helper image on that Docker daemon before running platform backups."));
            }

            var pull = await PullHelperImageAsync(imageConnector, command.PlatformAddress, helperImage, cancellationToken);
            if (pull.IsFailure(out var pullError))
                return LightResults.Result.Failure<string>(pullError);

            create = await TryCreateHelperAsync(containerConnector, command, helperImage, containerName, cancellationToken);
            if (!create.IsSuccess(out containerId, out createError))
                return LightResults.Result.Failure<string>(createError);
        }

        var started = await containerConnector.PatchAsync(
            new PatchContainerCommand(ContainerAction.START, command.PlatformAddress, [containerId]),
            cancellationToken);

        if (started.IsFailure())
        {
            await DeleteHelperAsync(containerConnector, command.PlatformAddress, containerId, CancellationToken.None);
            return LightResults.Result.Failure<string>(
                new Hosting.Common.ErrorTypes.BadGatewayError("Backup helper container failed to start."));
        }

        await Task.Delay(HelperStartProbeDelay, cancellationToken);
        var inspect = await containerConnector.InspectAsync(
            new InspectContainerCommand(command.PlatformAddress, containerId),
            cancellationToken);

        if (!inspect.IsSuccess(out var info) || info.State?.Running != true)
        {
            await DeleteHelperAsync(containerConnector, command.PlatformAddress, containerId, CancellationToken.None);
            var message = info?.State?.Error;
            return LightResults.Result.Failure<string>(
                new Hosting.Common.ErrorTypes.BadGatewayError(string.IsNullOrWhiteSpace(message)
                    ? "Backup helper container exited before it was ready."
                    : $"Backup helper container exited before it was ready: {message}"));
        }

        return containerId;
    }

    private static async Task<LightResults.Result<string>> TryCreateHelperAsync(
        IContainerConnector containerConnector,
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

        return await containerConnector.CreateAsync(
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
                AutoRemove: false,
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
                    "trap 'exit 0' TERM INT; while :; do sleep 3600; done"
                ]),
            cancellationToken);
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
        IContainerConnector connector,
        string platformAddress,
        string containerId,
        CancellationToken cancellationToken)
    {
        await connector.DeleteAsync(
            new DeleteContainerCommand([containerId], platformAddress, Volume: false, Force: true, Link: false),
            cancellationToken);
    }

    private static bool IsMissingHelperImageError(LightResults.IError error)
    {
        var message = error.Message;
        return message.Contains("No such image", StringComparison.OrdinalIgnoreCase)
               || message.Contains("image not found", StringComparison.OrdinalIgnoreCase)
               || message.Contains("not found: manifest", StringComparison.OrdinalIgnoreCase);
    }

    private static bool CanPullHelperImage(string helperImage)
    {
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

    [GeneratedRegex(@"\x1B\[[0-?]*[ -/]*[@-~]", RegexOptions.Compiled)]
    private static partial Regex AnsiRegex();

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
