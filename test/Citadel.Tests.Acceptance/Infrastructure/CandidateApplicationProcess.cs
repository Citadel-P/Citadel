using System.Diagnostics;
using System.Net;
using System.Net.Http.Headers;
using System.Net.Http.Json;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;

namespace Tests.Acceptance.Infrastructure;

internal sealed record CandidateApplicationOptions(
    string? JwtKey = "citadel-acceptance-signing-key-00000000000000000000000000000000",
    string? SecretEncryptionKey = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    int HttpTimeoutSeconds = 30)
{
    public static CandidateApplicationOptions FileBackedRecoveryAssets { get; } =
        new(JwtKey: null, SecretEncryptionKey: null);
}

internal sealed class CandidateApplicationProcess : IAsyncDisposable
{
    private readonly Process process;
    private readonly StringBuilder output = new();
    private readonly object outputLock = new();

    private CandidateApplicationProcess(
        Process process,
        Uri baseAddress,
        Uri grpcAddress,
        TimeSpan httpTimeout)
    {
        this.process = process;
        GrpcAddress = grpcAddress;
        Client = new HttpClient
        {
            BaseAddress = baseAddress,
            Timeout = httpTimeout
        };

        process.OutputDataReceived += CaptureOutput;
        process.ErrorDataReceived += CaptureOutput;
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
    }

    public HttpClient Client { get; }

    public Uri GrpcAddress { get; }

    public string Output => GetOutput();

    public static async Task<CandidateApplicationProcess> StartAsync(
        string connectionString,
        string workingDirectory,
        CancellationToken cancellationToken,
        CandidateApplicationOptions? options = null)
    {
        options ??= new CandidateApplicationOptions();
        var (httpPort, grpcPort) = GetAvailablePorts();
        var baseAddress = new Uri($"http://127.0.0.1:{httpPort}");
        var grpcAddress = new Uri($"http://127.0.0.1:{grpcPort}");
        var applicationPath = Path.Combine(AppContext.BaseDirectory, "Citadel.WebApi.dll");
        Assert.True(File.Exists(applicationPath), $"Candidate application not found at {applicationPath}.");

        var startInfo = new ProcessStartInfo("dotnet", $"\"{applicationPath}\"")
        {
            WorkingDirectory = workingDirectory,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };
        // Hosting.Common treats IntegrationTests as runtime execution, so the
        // real startup migration path is not mistaken for an MSBuild process.
        startInfo.Environment["ASPNETCORE_ENVIRONMENT"] = "IntegrationTests";
        startInfo.Environment["ASPNETCORE_CONTENTROOT"] = AppContext.BaseDirectory;
        startInfo.Environment["Kestrel__Endpoints__Http__Url"] = baseAddress.ToString();
        startInfo.Environment["Kestrel__Endpoints__Http__Protocols"] = "Http1";
        startInfo.Environment["Kestrel__Endpoints__Grpc__Url"] =
            grpcAddress.ToString();
        startInfo.Environment["Kestrel__Endpoints__Grpc__Protocols"] = "Http2";
        startInfo.Environment["ConnectionStrings__Postgres"] = connectionString;
        startInfo.Environment["Jwt__Issuer"] = baseAddress.ToString();
        startInfo.Environment["Jwt__Audience"] = baseAddress.ToString();
        startInfo.Environment["Jwt__Key"] = options.JwtKey ?? string.Empty;
        startInfo.Environment["Secrets__EncryptionKey"] =
            options.SecretEncryptionKey ?? string.Empty;

        var process = Process.Start(startInfo)
            ?? throw new InvalidOperationException("Failed to start the candidate Citadel process.");
        var candidate = new CandidateApplicationProcess(
            process,
            baseAddress,
            grpcAddress,
            TimeSpan.FromSeconds(options.HttpTimeoutSeconds));

        try
        {
            await candidate.WaitUntilHealthyAsync(cancellationToken);
            return candidate;
        }
        catch
        {
            await candidate.DisposeAsync();
            throw;
        }
    }

    public async Task AuthenticateAsAdminAsync(CancellationToken cancellationToken)
    {
        var login = await Client.PostAsJsonAsync(
            "/api/v1/authentication/login",
            new
            {
                emailOrName = "admin@citadel.local",
                password = "admin123"
            },
            cancellationToken);
        Assert.True(
            login.IsSuccessStatusCode,
            $"""
            Candidate login failed with HTTP {(int)login.StatusCode}.
            {await login.Content.ReadAsStringAsync(cancellationToken)}
            {Output}
            """);

        var loginPayload = await login.Content.ReadFromJsonAsync<JsonElement>(
            cancellationToken: cancellationToken);
        var accessToken = loginPayload.GetProperty("accessToken").GetString();
        Assert.False(string.IsNullOrWhiteSpace(accessToken));
        Client.DefaultRequestHeaders.Authorization =
            new AuthenticationHeaderValue("Bearer", accessToken);
    }

    public async ValueTask DisposeAsync()
    {
        Client.Dispose();

        if (!process.HasExited)
        {
            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch (InvalidOperationException) when (process.HasExited)
            {
            }
        }

        await process.WaitForExitAsync();
        process.Dispose();
    }

    private async Task WaitUntilHealthyAsync(CancellationToken cancellationToken)
    {
        var elapsed = Stopwatch.StartNew();

        while (elapsed.Elapsed < TimeSpan.FromSeconds(60))
        {
            cancellationToken.ThrowIfCancellationRequested();

            if (process.HasExited)
                throw new InvalidOperationException(
                    $"Candidate exited with code {process.ExitCode}.{Environment.NewLine}{GetOutput()}");

            using var probeTimeout =
                CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
            probeTimeout.CancelAfter(TimeSpan.FromSeconds(2));
            try
            {
                using var response = await Client.GetAsync("/health", probeTimeout.Token);
                if (response.StatusCode is HttpStatusCode.OK)
                    return;
            }
            catch (HttpRequestException)
            {
            }
            catch (TaskCanceledException) when (!cancellationToken.IsCancellationRequested)
            {
            }

            await Task.Delay(TimeSpan.FromMilliseconds(200), cancellationToken);
        }

        throw new TimeoutException(
            $"Candidate did not become healthy at {Client.BaseAddress}.{Environment.NewLine}{GetOutput()}");
    }

    private void CaptureOutput(object sender, DataReceivedEventArgs args)
    {
        if (args.Data is null)
            return;

        lock (outputLock)
            output.AppendLine(args.Data);
    }

    private string GetOutput()
    {
        lock (outputLock)
            return output.ToString();
    }

    private static (int Http, int Grpc) GetAvailablePorts()
    {
        var httpListener = new TcpListener(IPAddress.Loopback, 0);
        var grpcListener = new TcpListener(IPAddress.Loopback, 0);
        httpListener.Start();
        grpcListener.Start();
        var ports = (
            ((IPEndPoint)httpListener.LocalEndpoint).Port,
            ((IPEndPoint)grpcListener.LocalEndpoint).Port);
        httpListener.Stop();
        grpcListener.Stop();
        return ports;
    }
}
