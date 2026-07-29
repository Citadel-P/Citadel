using System.Security.Cryptography;
using System.Text;
using Application.Configs;
using Application.Features.Identity.Setup;
using Domain.Contracts.Interfaces;
using LightResults;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;

namespace Application.Services.Identity;

internal sealed class InitialAdministratorBootstrapService(
    IServiceScopeFactory scopeFactory,
    IOptions<BootstrapOptions> options,
    ILogger<InitialAdministratorBootstrapService> logger) : IHostedService
{
    private const long MaximumPasswordFileBytes = 1024;
    private const int PasswordReadBufferBytes = (int)MaximumPasswordFileBytes + 1;
    private static readonly UTF8Encoding StrictUtf8 = new(
        encoderShouldEmitUTF8Identifier: false,
        throwOnInvalidBytes: true);

    public async Task StartAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var cache = scope.ServiceProvider.GetRequiredService<ISetupStateCache>();
        var state = await unitOfWork.InstanceSetupState.GetAsync(cancellationToken)
            ?? throw new InvalidOperationException("Citadel setup state is unavailable.");

        cache.SetRequiresSetup(state.RequiresSetup);

        var configured = GetConfiguredSettings(options.Value);
        if (!state.RequiresSetup)
        {
            if (configured.Count > 0)
            {
                logger.LogInformation(
                    "Bootstrap administrator settings were ignored because Citadel setup is complete.");
            }

            return;
        }

        if (configured.Count == 0)
        {
            logger.LogInformation(
                "Citadel requires initial setup. Open the application to create the administrator.");
            return;
        }

        var missing = GetMissingSettings(options.Value);
        if (missing.Count > 0)
        {
            throw new InvalidOperationException(
                $"Incomplete bootstrap administrator configuration. Missing: {string.Join(", ", missing)}.");
        }

        var password = await ReadPasswordAsync(
            options.Value.AdminPasswordFile!,
            cancellationToken);

        var mediator = scope.ServiceProvider.GetRequiredService<IMediator>();
        var result = await mediator.Send(
            new InitializeCitadelUnattended(
                options.Value.AdminName!,
                options.Value.AdminEmail!,
                password),
            cancellationToken);

        if (result.IsFailure(out var error))
        {
            throw new InvalidOperationException(
                $"Unattended administrator bootstrap failed: {error.Message}");
        }

        logger.LogInformation("Citadel initial administrator was created by unattended bootstrap.");
    }

    public Task StopAsync(CancellationToken cancellationToken) => Task.CompletedTask;

    private static List<string> GetConfiguredSettings(BootstrapOptions options)
    {
        var configured = new List<string>(3);
        if (!string.IsNullOrWhiteSpace(options.AdminName))
            configured.Add(nameof(options.AdminName));
        if (!string.IsNullOrWhiteSpace(options.AdminEmail))
            configured.Add(nameof(options.AdminEmail));
        if (!string.IsNullOrWhiteSpace(options.AdminPasswordFile))
            configured.Add(nameof(options.AdminPasswordFile));
        return configured;
    }

    private static List<string> GetMissingSettings(BootstrapOptions options)
    {
        var missing = new List<string>(3);
        if (string.IsNullOrWhiteSpace(options.AdminName))
            missing.Add($"{BootstrapOptions.SectionName}:{nameof(options.AdminName)}");
        if (string.IsNullOrWhiteSpace(options.AdminEmail))
            missing.Add($"{BootstrapOptions.SectionName}:{nameof(options.AdminEmail)}");
        if (string.IsNullOrWhiteSpace(options.AdminPasswordFile))
            missing.Add($"{BootstrapOptions.SectionName}:{nameof(options.AdminPasswordFile)}");
        return missing;
    }

    internal static async Task<string> ReadPasswordAsync(
        string path,
        CancellationToken cancellationToken)
    {
        if (!Path.IsPathFullyQualified(path))
            throw new InvalidOperationException("Bootstrap:AdminPasswordFile must be an absolute path.");

        var file = new FileInfo(path);
        const FileAttributes unsupportedAttributes =
            FileAttributes.Directory
            | FileAttributes.Device
            | FileAttributes.ReparsePoint;
        if (!file.Exists || (file.Attributes & unsupportedAttributes) != 0)
        {
            throw new InvalidOperationException(
                "Bootstrap:AdminPasswordFile must reference a readable regular file.");
        }

        if (file.Length > MaximumPasswordFileBytes)
            throw new InvalidOperationException("Bootstrap:AdminPasswordFile exceeds the 1 KiB limit.");

        var bytes = new byte[PasswordReadBufferBytes];
        int bytesRead = 0;
        try
        {
            await using var stream = new FileStream(
                path,
                FileMode.Open,
                FileAccess.Read,
                FileShare.Read,
                bufferSize: PasswordReadBufferBytes,
                FileOptions.Asynchronous | FileOptions.SequentialScan);
            if (!stream.CanSeek)
            {
                throw new InvalidOperationException(
                    "Bootstrap:AdminPasswordFile must reference a readable regular file.");
            }

            while (bytesRead < bytes.Length)
            {
                var read = await stream.ReadAsync(
                    bytes.AsMemory(bytesRead, bytes.Length - bytesRead),
                    cancellationToken);
                if (read == 0)
                    break;

                bytesRead += read;
            }

            if (bytesRead > MaximumPasswordFileBytes)
            {
                throw new InvalidOperationException(
                    "Bootstrap:AdminPasswordFile exceeds the 1 KiB limit.");
            }

            var content = bytes.AsSpan(0, bytesRead);
            if (content.StartsWith(Encoding.UTF8.Preamble))
                content = content[Encoding.UTF8.Preamble.Length..];

            string password;
            try
            {
                password = StrictUtf8.GetString(content);
            }
            catch (DecoderFallbackException exception)
            {
                throw new InvalidOperationException(
                    "Bootstrap:AdminPasswordFile must contain valid UTF-8 text.",
                    exception);
            }

            if (password.EndsWith("\r\n", StringComparison.Ordinal))
                password = password[..^2];
            else if (password.EndsWith('\n'))
                password = password[..^1];

            if (password.Contains('\r') || password.Contains('\n'))
            {
                throw new InvalidOperationException(
                    "Bootstrap:AdminPasswordFile must contain a single password line.");
            }

            return password;
        }
        finally
        {
            CryptographicOperations.ZeroMemory(bytes);
        }
    }
}
