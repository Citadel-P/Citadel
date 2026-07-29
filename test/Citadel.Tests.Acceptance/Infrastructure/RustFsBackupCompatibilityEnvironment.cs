using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;
using DotNet.Testcontainers.Volumes;
using Minio;
using Minio.DataModel.Args;
using System.Diagnostics;
using System.Formats.Tar;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;

namespace Tests.Acceptance.Infrastructure;

internal sealed class RustFsBackupCompatibilityEnvironment : IAsyncDisposable
{
    private const ushort CoreHttpPort = 8000;
    private const ushort RustFsPort = 9000;
    private const string DockerImage = "docker:27.5.1-dind";
    private const string RustFsContainerName = "citadel-acceptance-rustfs";
    // RustFS 1.0.0-beta.10 is pinned so compatibility does not drift with latest.
    private const string RustFsImage =
        "rustfs/rustfs@sha256:60f4f2f41ce95216f8cac676e69f9d90c0bfec458a3bc7fd7fb9b7c2452ac57a";

    public const string AccessKey = "citadel-acceptance";
    public const string SecretKey = "citadel-acceptance-secret-2026";

    private readonly IVolume dockerSocketVolume;
    private readonly IVolume dockerDataVolume;
    private readonly IVolume coreDataVolume;
    private readonly IContainer dockerDaemon;
    private readonly IContainer core;
    private readonly string helperImage;
    private IMinioClient? objectStorage;

    private RustFsBackupCompatibilityEnvironment(
        string coreImage,
        string postgresConnectionString)
    {
        helperImage = $"citadel-acceptance-backup-helper:{Guid.NewGuid():N}";
        dockerSocketVolume = new VolumeBuilder().Build();
        dockerDataVolume = new VolumeBuilder().Build();
        coreDataVolume = new VolumeBuilder().Build();

        dockerDaemon = new ContainerBuilder(DockerImage)
            .WithPrivileged(true)
            .WithEnvironment("DOCKER_TLS_CERTDIR", string.Empty)
            .WithPortBinding(RustFsPort, assignRandomHostPort: true)
            .WithVolumeMount(dockerSocketVolume, "/var/run")
            .WithVolumeMount(dockerDataVolume, "/var/lib/docker")
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilCommandIsCompleted(
                    "docker",
                    "info"))
            .Build();

        core = new ContainerBuilder(coreImage)
            .WithPortBinding(CoreHttpPort, assignRandomHostPort: true)
            .WithEnvironment("ASPNETCORE_ENVIRONMENT", "Production")
            .WithEnvironment(
                "ConnectionStrings__Postgres",
                postgresConnectionString)
            .WithEnvironment("Jwt__Issuer", "http://citadel-core")
            .WithEnvironment("Jwt__Audience", "http://citadel-core")
            .WithEnvironment(
                "Jwt__Key",
                "citadel-rustfs-compatibility-signing-key-0000000000000000000")
            .WithEnvironment(
                "Secrets__EncryptionKey",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
            .WithEnvironment("DOCKER_HOST", "unix:///var/run/docker.sock")
            .WithEnvironment("VolumeBrowser__HelperImage", helperImage)
            .WithVolumeMount(dockerSocketVolume, "/var/run")
            .WithVolumeMount(coreDataVolume, "/app/data")
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilHttpRequestIsSucceeded(
                    request => request
                        .ForPort(CoreHttpPort)
                        .ForPath("/health")))
            .Build();
    }

    public HttpClient Client { get; private set; } = null!;

    public string PlatformS3Endpoint { get; private set; } = null!;

    public static async Task<RustFsBackupCompatibilityEnvironment> StartAsync(
        string coreImage,
        string postgresConnectionString,
        CancellationToken cancellationToken)
    {
        Assert.False(
            string.IsNullOrWhiteSpace(coreImage),
            "Set CITADEL_ACCEPTANCE_CORE_IMAGE to the Core image under test.");

        var environment = new RustFsBackupCompatibilityEnvironment(
            coreImage,
            postgresConnectionString);
        try
        {
            await environment.dockerSocketVolume.CreateAsync(cancellationToken);
            await environment.dockerDataVolume.CreateAsync(cancellationToken);
            await environment.coreDataVolume.CreateAsync(cancellationToken);
            await environment.dockerDaemon.StartAsync(cancellationToken);

            await environment.ImportHelperImageAsync(
                coreImage,
                cancellationToken);
            await environment.StartRustFsAsync(cancellationToken);
            await environment.core.StartAsync(cancellationToken);

            environment.Client = new HttpClient
            {
                BaseAddress = new Uri(
                    $"http://127.0.0.1:{environment.core.GetMappedPublicPort(CoreHttpPort)}"),
                Timeout = TimeSpan.FromMinutes(5)
            };
            return environment;
        }
        catch
        {
            await environment.DisposeAsync();
            throw;
        }
    }

    public async Task AuthenticateAsAdminAsync(
        CancellationToken cancellationToken)
    {
        await InitialAdministratorSession.AuthenticateAsync(
            Client,
            "Core image",
            cancellationToken);
    }

    public async Task CreateBucketAsync(
        string bucket,
        CancellationToken cancellationToken)
    {
        var client = GetObjectStorageClient();
        var exists = await client.BucketExistsAsync(
            new BucketExistsArgs().WithBucket(bucket),
            cancellationToken);
        if (!exists)
        {
            await client.MakeBucketAsync(
                new MakeBucketArgs().WithBucket(bucket),
                cancellationToken);
        }
    }

    public async Task PutObjectAsync(
        string bucket,
        string objectName,
        byte[] content,
        CancellationToken cancellationToken)
    {
        await using var stream = new MemoryStream(content, writable: false);
        await GetObjectStorageClient().PutObjectAsync(
            new PutObjectArgs()
                .WithBucket(bucket)
                .WithObject(objectName)
                .WithStreamData(stream)
                .WithObjectSize(content.LongLength),
            cancellationToken);
    }

    public async Task<IReadOnlyList<string>> ListObjectNamesAsync(
        string bucket,
        string prefix,
        CancellationToken cancellationToken)
    {
        var names = new List<string>();
        var args = new ListObjectsArgs()
            .WithBucket(bucket)
            .WithPrefix(prefix)
            .WithRecursive(true);

        await foreach (var item in GetObjectStorageClient()
            .ListObjectsEnumAsync(args, cancellationToken))
        {
            names.Add(item.Key);
        }

        return names;
    }

    public async Task SeedVolumeAsync(
        string volumeName,
        byte[] content,
        CancellationToken cancellationToken)
    {
        await RunDockerAsync(
            ["volume", "create", volumeName],
            cancellationToken);
        var seedContainer = $"citadel-volume-seed-{Guid.NewGuid():N}";
        try
        {
            await RunDockerAsync(
                [
                    "create",
                    "--name",
                    seedContainer,
                    "--volume",
                    $"{volumeName}:/data",
                    "busybox:1.36.1"
                ],
                cancellationToken);
            await CopyPayloadToNestedContainerAsync(
                content,
                $"{seedContainer}:/data",
                cancellationToken);
        }
        finally
        {
            await dockerDaemon.ExecAsync(
                ["docker", "rm", "--force", seedContainer],
                CancellationToken.None);
        }
    }

    public async Task<byte[]> ReadVolumePayloadAsync(
        string volumeName,
        CancellationToken cancellationToken)
    {
        var result = await RunDockerAsync(
            [
                "run",
                "--rm",
                "--volume",
                $"{volumeName}:/data:ro",
                "busybox:1.36.1",
                "base64",
                "/data/payload.bin"
            ],
            cancellationToken);

        return Convert.FromBase64String(
            string.Concat(
                result.Stdout
                    .Split(
                        ['\r', '\n'],
                        StringSplitOptions.RemoveEmptyEntries)));
    }

    public async Task<string> GetDiagnosticsAsync(
        CancellationToken cancellationToken)
    {
        var (coreStdout, coreStderr) = await core.GetLogsAsync(
            DateTime.UnixEpoch,
            DateTime.UtcNow,
            timestampsEnabled: false,
            cancellationToken);
        var dockerContainers = await dockerDaemon.ExecAsync(
            ["docker", "ps", "--all"],
            cancellationToken);
        var rustFsLogs = await dockerDaemon.ExecAsync(
            ["docker", "logs", RustFsContainerName],
            cancellationToken);

        return
            $"Core stdout:{Environment.NewLine}{coreStdout}"
            + $"{Environment.NewLine}Core stderr:{Environment.NewLine}{coreStderr}"
            + $"{Environment.NewLine}Platform containers:{Environment.NewLine}{dockerContainers.Stdout}"
            + $"{Environment.NewLine}RustFS stdout:{Environment.NewLine}{rustFsLogs.Stdout}"
            + $"{Environment.NewLine}RustFS stderr:{Environment.NewLine}{rustFsLogs.Stderr}";
    }

    public async ValueTask DisposeAsync()
    {
        Client?.Dispose();
        objectStorage?.Dispose();

        await core.DisposeAsync();
        await dockerDaemon.DisposeAsync();
        await coreDataVolume.DisposeAsync();
        await dockerDataVolume.DisposeAsync();
        await dockerSocketVolume.DisposeAsync();
    }

    private IMinioClient GetObjectStorageClient()
    {
        Assert.NotNull(objectStorage);
        return objectStorage;
    }

    private async Task StartRustFsAsync(
        CancellationToken cancellationToken)
    {
        await RunDockerAsync(["pull", RustFsImage], cancellationToken);
        await RunDockerAsync(
            ["pull", "busybox:1.36.1"],
            cancellationToken);
        await RunDockerAsync(
            [
                "run",
                "--detach",
                "--name",
                RustFsContainerName,
                "--publish",
                $"{RustFsPort}:{RustFsPort}",
                "--env",
                $"RUSTFS_ACCESS_KEY={AccessKey}",
                "--env",
                $"RUSTFS_SECRET_KEY={SecretKey}",
                RustFsImage
            ],
            cancellationToken);

        var gateway = await RunDockerAsync(
            [
                "network",
                "inspect",
                "bridge",
                "--format",
                "{{(index .IPAM.Config 0).Gateway}}"
            ],
            cancellationToken);
        PlatformS3Endpoint =
            $"http://{gateway.Stdout.Trim()}:{RustFsPort}";

        var hostEndpoint = new Uri(
            $"http://127.0.0.1:{dockerDaemon.GetMappedPublicPort(RustFsPort)}");
        objectStorage = new MinioClient()
            .WithEndpoint(hostEndpoint.Host, hostEndpoint.Port)
            .WithCredentials(AccessKey, SecretKey)
            .Build();

        var timeout = Stopwatch.StartNew();
        while (timeout.Elapsed < TimeSpan.FromMinutes(2))
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                await objectStorage.ListBucketsAsync(cancellationToken);
                return;
            }
            catch
            {
                await Task.Delay(
                    TimeSpan.FromMilliseconds(500),
                    cancellationToken);
            }
        }

        throw new TimeoutException(
            $"RustFS did not become ready.{Environment.NewLine}"
            + await GetRustFsLogsAsync(CancellationToken.None));
    }

    private async Task ImportHelperImageAsync(
        string coreImage,
        CancellationToken cancellationToken)
    {
        await RunProcessAsync(
            "docker",
            ["image", "tag", coreImage, helperImage],
            cancellationToken);

        try
        {
            using var save = StartProcess(
                "docker",
                ["image", "save", helperImage],
                redirectStandardInput: false);
            using var load = StartProcess(
                "docker",
                ["exec", "--interactive", dockerDaemon.Id, "docker", "image", "load"],
                redirectStandardInput: true);

            var saveErrorTask = save.StandardError.ReadToEndAsync(
                cancellationToken);
            var loadOutputTask = load.StandardOutput.ReadToEndAsync(
                cancellationToken);
            var loadErrorTask = load.StandardError.ReadToEndAsync(
                cancellationToken);

            await save.StandardOutput.BaseStream.CopyToAsync(
                load.StandardInput.BaseStream,
                cancellationToken);
            load.StandardInput.Close();

            await Task.WhenAll(
                save.WaitForExitAsync(cancellationToken),
                load.WaitForExitAsync(cancellationToken));
            var saveError = await saveErrorTask;
            var loadOutput = await loadOutputTask;
            var loadError = await loadErrorTask;

            Assert.True(
                save.ExitCode == 0 && load.ExitCode == 0,
                $"""
                Importing the candidate backup helper image failed.
                docker save exit code: {save.ExitCode}
                docker save stderr: {saveError}
                docker load exit code: {load.ExitCode}
                docker load stdout: {loadOutput}
                docker load stderr: {loadError}
                """);
        }
        finally
        {
            await RunProcessAsync(
                "docker",
                ["image", "remove", helperImage],
                CancellationToken.None,
                assertSuccess: false);
        }
    }

    private async Task CopyPayloadToNestedContainerAsync(
        byte[] content,
        string destination,
        CancellationToken cancellationToken)
    {
        await using var archive = new MemoryStream();
        await using var payload = new MemoryStream(content, writable: false);
        using (var writer = new TarWriter(archive, leaveOpen: true))
        {
            writer.WriteEntry(
                new PaxTarEntry(
                    TarEntryType.RegularFile,
                    "payload.bin")
                {
                    DataStream = payload
                });
        }

        archive.Position = 0;
        using var process = StartProcess(
            "docker",
            [
                "exec",
                "--interactive",
                dockerDaemon.Id,
                "docker",
                "cp",
                "-",
                destination
            ],
            redirectStandardInput: true);
        var outputTask = process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var errorTask = process.StandardError.ReadToEndAsync(
            cancellationToken);

        await archive.CopyToAsync(
            process.StandardInput.BaseStream,
            cancellationToken);
        process.StandardInput.Close();
        await process.WaitForExitAsync(cancellationToken);

        var output = await outputTask;
        var error = await errorTask;
        Assert.True(
            process.ExitCode == 0,
            $"""
            Copying the volume fixture into nested Docker failed with exit code {process.ExitCode}.
            stdout: {output}
            stderr: {error}
            """);
    }

    private async Task<(string Stdout, string Stderr)> RunDockerAsync(
        IReadOnlyList<string> arguments,
        CancellationToken cancellationToken)
    {
        var result = await dockerDaemon.ExecAsync(
            ["docker", .. arguments],
            cancellationToken);
        Assert.True(
            result.ExitCode == 0,
            $"""
            Nested Docker command failed with exit code {result.ExitCode ?? -1L}.
            docker {string.Join(' ', arguments)}
            stdout: {result.Stdout}
            stderr: {result.Stderr}
            """);
        return (result.Stdout, result.Stderr);
    }

    private async Task<string> GetRustFsLogsAsync(
        CancellationToken cancellationToken)
    {
        var result = await dockerDaemon.ExecAsync(
            ["docker", "logs", RustFsContainerName],
            cancellationToken);
        return result.Stdout + Environment.NewLine + result.Stderr;
    }

    private static async Task RunProcessAsync(
        string fileName,
        IReadOnlyList<string> arguments,
        CancellationToken cancellationToken,
        bool assertSuccess = true)
    {
        using var process = StartProcess(
            fileName,
            arguments,
            redirectStandardInput: false);
        var outputTask = process.StandardOutput.ReadToEndAsync(
            cancellationToken);
        var errorTask = process.StandardError.ReadToEndAsync(
            cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        var output = await outputTask;
        var error = await errorTask;

        if (assertSuccess)
        {
            Assert.True(
                process.ExitCode == 0,
                $"""
                {fileName} {string.Join(' ', arguments)} failed with exit code {process.ExitCode}.
                stdout: {output}
                stderr: {error}
                """);
        }
    }

    private static Process StartProcess(
        string fileName,
        IReadOnlyList<string> arguments,
        bool redirectStandardInput)
    {
        var startInfo = new ProcessStartInfo(fileName)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardInput = redirectStandardInput,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        foreach (var argument in arguments)
            startInfo.ArgumentList.Add(argument);

        return Process.Start(startInfo)
            ?? throw new InvalidOperationException(
                $"Failed to start {fileName}.");
    }
}
