using System.Diagnostics;
using DotNet.Testcontainers.Builders;
using DotNet.Testcontainers.Containers;

namespace Tests.Acceptance.Infrastructure;

internal sealed class StandaloneDockerDaemon : IAsyncDisposable
{
    private const ushort DockerApiPort = 2375;
    private const string Image = "docker:27.5.1-dind";

    private readonly IContainer container;

    private StandaloneDockerDaemon(IContainer container, string dockerHost)
    {
        this.container = container;
        DockerHost = dockerHost;
    }

    public string DockerHost { get; }

    public static async Task<StandaloneDockerDaemon> StartAsync(
        CancellationToken cancellationToken)
    {
        var container = new ContainerBuilder(Image)
            .WithPrivileged(true)
            .WithEnvironment("DOCKER_TLS_CERTDIR", string.Empty)
            .WithPortBinding(DockerApiPort, assignRandomHostPort: true)
            .WithWaitStrategy(
                Wait.ForUnixContainer().UntilCommandIsCompleted(
                    "docker",
                    "info"))
            .Build();

        try
        {
            await container.StartAsync(cancellationToken);
            return new StandaloneDockerDaemon(
                container,
                $"tcp://127.0.0.1:{container.GetMappedPublicPort(DockerApiPort)}");
        }
        catch
        {
            await container.DisposeAsync();
            throw;
        }
    }

    public async Task<string> RunAsync(
        CancellationToken cancellationToken,
        bool throwOnFailure,
        params string[] arguments)
    {
        var startInfo = new ProcessStartInfo("docker")
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        startInfo.Environment["DOCKER_HOST"] = DockerHost;
        foreach (var argument in arguments)
            startInfo.ArgumentList.Add(argument);

        using var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start Docker.");
        var outputTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var errorTask = process.StandardError.ReadToEndAsync(cancellationToken);
        await process.WaitForExitAsync(cancellationToken);
        var output = await outputTask;
        var error = await errorTask;

        if (throwOnFailure)
        {
            Assert.True(
                process.ExitCode == 0,
                $"docker {string.Join(' ', arguments)} failed with exit code {process.ExitCode}: {error}");
        }

        return string.Concat(output, error);
    }

    public ValueTask DisposeAsync() => container.DisposeAsync();
}
