using Application.Configs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Domain.Entities.ResourceBindings;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Options;
using System.Runtime.InteropServices;

namespace Application.Services.Backups;

public interface IBackupRepositoryDestinationService
{
    ValueTask<Result<BackupRepositoryValidation>> ValidateAsync(
        Guid repositoryId,
        BackupExecutionContext context,
        CancellationToken cancellationToken);

    ValueTask<Result> InitializeAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken);
    ValueTask<Result> CheckAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken);
    ValueTask<Result> PruneAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken);
}

internal interface IResticEnvironmentBuilder
{
    Task<Result<ResticRepositoryEnvironment>> BuildAsync(
        IUnitOfWork uow,
        BackupRepository repository,
        BackupExecutionContext context,
        CancellationToken cancellationToken);
}

internal sealed class BackupRepositoryDestinationService(
    IServiceScopeFactory scopeFactory,
    IResticEnvironmentBuilder environmentBuilder,
    IResticProcessRunner processRunner,
    IPlatformResticRunner platformResticRunner,
    IPlatformContainerCache platformContainerCache,
    IOptions<BackupOptions> backupOptions)
    : IBackupRepositoryDestinationService
{
    private readonly BackupOptions options = backupOptions.Value;

    public async ValueTask<Result<BackupRepositoryValidation>> ValidateAsync(
        Guid repositoryId,
        BackupExecutionContext context,
        CancellationToken cancellationToken)
    {
        var load = await LoadRepositoryAsync(repositoryId, context, cancellationToken);
        if (!load.IsSuccess(out var loaded, out var loadError))
            return Result.Failure<BackupRepositoryValidation>(loadError!);

        await using var environment = loaded.Environment;
        var result = await RunResticAsync(environment, ["snapshots", "--json"], cancellationToken);
        var validation = ToValidation(repositoryId, context, result, DateTimeOffset.UtcNow);

        await PersistValidationAsync(validation, markChecked: false, markPruned: false, cancellationToken);
        return Result.Success(validation);
    }

    public async ValueTask<Result> InitializeAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken)
    {
        var ownerRunId = Guid.CreateVersion7();
        var lease = await AcquireLeaseAsync(repositoryId, "Initialize", ownerRunId, cancellationToken);
        if (!lease.IsSuccess())
            return Result.Failure(lease.Errors);

        try
        {
            var load = await LoadRepositoryAsync(repositoryId, context, cancellationToken);
            if (!load.IsSuccess(out var loaded, out var loadError))
                return Result.Failure(loadError!);

            await using var environment = loaded.Environment;
            var precheck = await RunResticAsync(environment, ["snapshots", "--json"], cancellationToken);
            if (precheck.ExitCode == 0)
            {
                var readyValidation = ToValidation(repositoryId, context, precheck, DateTimeOffset.UtcNow);
                await PersistValidationAsync(readyValidation, markChecked: false, markPruned: false, cancellationToken);
                return Result.Success();
            }

            var precheckStatus = Classify(precheck);
            if (precheckStatus is not BackupRepositoryValidationStatus.Uninitialized)
            {
                var failedValidation = ToValidation(repositoryId, context, precheck, DateTimeOffset.UtcNow);
                await PersistValidationAsync(failedValidation, markChecked: false, markPruned: false, cancellationToken);
                return Result.Failure(new BadRequestError(failedValidation.LastErrorMessage ?? "Backup repository cannot be initialized."));
            }

            var init = await RunResticAsync(environment, ["init", "--json"], cancellationToken);
            if (init.ExitCode != 0)
            {
                var failedValidation = ToValidation(repositoryId, context, init, DateTimeOffset.UtcNow);
                await PersistValidationAsync(failedValidation, markChecked: false, markPruned: false, cancellationToken);
                return Result.Failure(new BadRequestError(failedValidation.LastErrorMessage ?? "Backup repository initialization failed."));
            }

            var validationResult = await RunResticAsync(environment, ["snapshots", "--json"], cancellationToken);
            var validation = ToValidation(repositoryId, context, validationResult, DateTimeOffset.UtcNow);
            await PersistValidationAsync(validation, markChecked: false, markPruned: false, cancellationToken);

            return validation.Status == BackupRepositoryValidationStatus.Ready
                ? Result.Success()
                : Result.Failure(new BadRequestError(validation.LastErrorMessage ?? "Backup repository initialization could not be verified."));
        }
        finally
        {
            await ReleaseLeaseAsync(repositoryId, ownerRunId, CancellationToken.None);
        }
    }

    public async ValueTask<Result> CheckAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken)
    {
        var ownerRunId = Guid.CreateVersion7();
        var lease = await AcquireLeaseAsync(repositoryId, "Check", ownerRunId, cancellationToken);
        if (!lease.IsSuccess())
            return Result.Failure(lease.Errors);

        try
        {
            var load = await LoadRepositoryAsync(repositoryId, context, cancellationToken);
            if (!load.IsSuccess(out var loaded, out var loadError))
                return Result.Failure(loadError!);

            await using var environment = loaded.Environment;
            var result = await RunResticAsync(environment, ["check", "--json"], cancellationToken);
            var validation = ToValidation(repositoryId, context, result, DateTimeOffset.UtcNow);
            await PersistValidationAsync(validation, markChecked: result.ExitCode == 0, markPruned: false, cancellationToken);

            return result.ExitCode == 0
                ? Result.Success()
                : Result.Failure(new BadRequestError(validation.LastErrorMessage ?? "Backup repository check failed."));
        }
        finally
        {
            await ReleaseLeaseAsync(repositoryId, ownerRunId, CancellationToken.None);
        }
    }

    public async ValueTask<Result> PruneAsync(Guid repositoryId, BackupExecutionContext context, CancellationToken cancellationToken)
    {
        var ownerRunId = Guid.CreateVersion7();
        var lease = await AcquireLeaseAsync(repositoryId, "Prune", ownerRunId, cancellationToken);
        if (!lease.IsSuccess())
            return Result.Failure(lease.Errors);

        try
        {
            var load = await LoadRepositoryAsync(repositoryId, context, cancellationToken);
            if (!load.IsSuccess(out var loaded, out var loadError))
                return Result.Failure(loadError!);

            await using var environment = loaded.Environment;
            var result = await RunResticAsync(environment, ["prune", "--json"], cancellationToken);
            var validation = ToValidation(repositoryId, context, result, DateTimeOffset.UtcNow);
            await PersistValidationAsync(validation, markChecked: false, markPruned: result.ExitCode == 0, cancellationToken);

            return result.ExitCode == 0
                ? Result.Success()
                : Result.Failure(new BadRequestError(validation.LastErrorMessage ?? "Backup repository prune failed."));
        }
        finally
        {
            await ReleaseLeaseAsync(repositoryId, ownerRunId, CancellationToken.None);
        }
    }

    private async Task<Result<LoadedBackupRepository>> LoadRepositoryAsync(
        Guid repositoryId,
        BackupExecutionContext context,
        CancellationToken cancellationToken)
    {
        if (!options.Enabled)
            return Result.Failure<LoadedBackupRepository>(new BadRequestError("Backups are disabled."));

        try
        {
            context.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<LoadedBackupRepository>(new BadRequestError(ex.Message));
        }

        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repository = await uow.BackupRepositories.GetAsync(repositoryId, cancellationToken);
        if (repository is null)
            return Result.Failure<LoadedBackupRepository>(new NotFoundError("Backup repository not found."));

        var environment = await environmentBuilder.BuildAsync(uow, repository, context, cancellationToken);
        return environment.IsSuccess(out var value, out var error)
            ? new LoadedBackupRepository(repository, value)
            : Result.Failure<LoadedBackupRepository>(error!);
    }

    private async Task<Result> AcquireLeaseAsync(Guid repositoryId, string operationType, Guid ownerRunId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var now = DateTimeOffset.UtcNow;
        var acquireResult = await uow.BackupRepositoryLeases.TryAcquireForExistingRepositoryAsync(
            repositoryId,
            operationType,
            ownerRunId,
            now.AddSeconds(Math.Max(30, options.RepositoryLeaseSeconds)),
            now,
            cancellationToken);
        await uow.CommitAsync(cancellationToken);

        return acquireResult switch
        {
            BackupRepositoryLeaseAcquireResult.Acquired => Result.Success(),
            BackupRepositoryLeaseAcquireResult.NotFound => Result.Failure(new NotFoundError("Backup repository not found.")),
            _ => Result.Failure(new ConflictError("Backup repository already has an active operation."))
        };
    }

    private async Task ReleaseLeaseAsync(Guid repositoryId, Guid ownerRunId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositoryLeases.ReleaseAsync(repositoryId, ownerRunId, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task PersistValidationAsync(
        BackupRepositoryValidation validation,
        bool markChecked,
        bool markPruned,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await uow.BackupRepositories.ApplyValidationResultAsync(validation, markChecked, markPruned, cancellationToken);
        await uow.CommitAsync(cancellationToken);
    }

    private async Task<ResticOperationResult> RunResticAsync(
        ResticRepositoryEnvironment environment,
        IReadOnlyList<string> operationArguments,
        CancellationToken cancellationToken)
    {
        var args = environment.CommonArguments.Concat(operationArguments).ToArray();
        var stdout = new List<string>();
        var stderr = new List<string>();
        int? exitCode = null;

        var stream = BuildResticStream(environment, args, cancellationToken);
        await foreach (var item in stream)
        {
            if (item.ExitCode.HasValue)
                exitCode = item.ExitCode.Value;
            else if (item.Stream == ResticProcessStream.StdErr)
                stderr.Add(item.Message ?? string.Empty);
            else
                stdout.Add(item.Message ?? string.Empty);
        }

        return new ResticOperationResult(exitCode ?? -1, string.Join('\n', stdout), string.Join('\n', stderr));
    }

    private IAsyncEnumerable<ResticProcessEvent> BuildResticStream(
        ResticRepositoryEnvironment environment,
        IReadOnlyList<string> args,
        CancellationToken cancellationToken)
    {
        if (environment.Context.Location == BackupExecutionLocation.Core)
        {
            return processRunner.RunAsync(
                new ResticProcessCommand(
                    options.ResticPath,
                    args,
                    environment.Environment,
                    environment.WorkingDirectory,
                    TimeSpan.FromSeconds(Math.Max(5, options.DefaultTimeoutSeconds)),
                    environment.RedactionValues,
                    Math.Max(1024, options.MaxLogLineBytes)),
                cancellationToken);
        }

        if (!environment.Context.PlatformId.HasValue)
            return SingleError("Platform backup execution requires a platform ID.");

        if (!platformContainerCache.TryGetCacheEntry(environment.Context.PlatformId.Value, out var platform, out var cacheError))
        {
            return SingleError(cacheError.Message);
        }

        return platformResticRunner.RunAsync(
            new PlatformResticCommand(
                platform.Id,
                platform.Address,
                platform.ConnectorType,
                ResticExecutable,
                args,
                environment.RemoteEnvironment,
                TimeSpan.FromSeconds(Math.Max(5, options.DefaultTimeoutSeconds)),
                environment.RedactionValues,
                Math.Max(1024, options.MaxLogLineBytes),
                SourceVolumeName: null,
                RepositoryHostPath: environment.PlatformRepositoryHostPath,
                NetworkMode: environment.RepositoryRequiresNetwork ? null : "none"),
            cancellationToken);
    }

    private static async IAsyncEnumerable<ResticProcessEvent> SingleError(string message)
    {
        await Task.Yield();
        yield return new ResticProcessEvent(ResticProcessStream.StdErr, message);
        yield return new ResticProcessEvent(ResticProcessStream.Exit, ExitCode: 1);
    }

    private const string ResticExecutable = "restic";

    private static BackupRepositoryValidation ToValidation(
        Guid repositoryId,
        BackupExecutionContext context,
        ResticOperationResult result,
        DateTimeOffset now)
    {
        var status = Classify(result);
        var message = status == BackupRepositoryValidationStatus.Ready
            ? null
            : SanitizeError(result);

        return new BackupRepositoryValidation(
            repositoryId,
            context.Location,
            context.PlatformId,
            status,
            now,
            status == BackupRepositoryValidationStatus.Ready ? null : ToErrorCode(status),
            message);
    }

    private static BackupRepositoryValidationStatus Classify(ResticOperationResult result)
    {
        if (result.ExitCode == 0)
            return BackupRepositoryValidationStatus.Ready;

        var text = $"{result.StdErr}\n{result.StdOut}";
        if (text.Contains("wrong password", StringComparison.OrdinalIgnoreCase)
            || text.Contains("invalid password", StringComparison.OrdinalIgnoreCase)
            || text.Contains("password is incorrect", StringComparison.OrdinalIgnoreCase))
        {
            return BackupRepositoryValidationStatus.InvalidPassword;
        }

        if (text.Contains("repository does not exist", StringComparison.OrdinalIgnoreCase)
            || text.Contains("config file does not exist", StringComparison.OrdinalIgnoreCase)
            || text.Contains("unable to open config file", StringComparison.OrdinalIgnoreCase)
            || text.Contains("Is there a repository", StringComparison.OrdinalIgnoreCase))
        {
            return BackupRepositoryValidationStatus.Uninitialized;
        }

        if (text.Contains("invalid", StringComparison.OrdinalIgnoreCase)
            || text.Contains("malformed", StringComparison.OrdinalIgnoreCase)
            || text.Contains("unsupported", StringComparison.OrdinalIgnoreCase))
        {
            return BackupRepositoryValidationStatus.InvalidConfiguration;
        }

        return BackupRepositoryValidationStatus.Unavailable;
    }

    private static string ToErrorCode(BackupRepositoryValidationStatus status)
        => status switch
        {
            BackupRepositoryValidationStatus.Uninitialized => "backup.repository.uninitialized",
            BackupRepositoryValidationStatus.InvalidPassword => "backup.repository.invalid_password",
            BackupRepositoryValidationStatus.InvalidConfiguration => "backup.repository.invalid_configuration",
            BackupRepositoryValidationStatus.Unavailable => "backup.repository.unavailable",
            _ => "backup.repository.validation_failed"
        };

    private static string SanitizeError(ResticOperationResult result)
    {
        var text = string.IsNullOrWhiteSpace(result.StdErr) ? result.StdOut : result.StdErr;
        return string.IsNullOrWhiteSpace(text)
            ? $"Restic exited with code {result.ExitCode}."
            : text.Trim();
    }
}

internal sealed class ResticEnvironmentBuilder(
    ISecretValueProtector secretValueProtector,
    IExternalSecretProviderClient externalSecretProviderClient,
    IOptions<BackupOptions> backupOptions)
    : IResticEnvironmentBuilder
{
    private readonly BackupOptions options = backupOptions.Value;

    public async Task<Result<ResticRepositoryEnvironment>> BuildAsync(
        IUnitOfWork uow,
        BackupRepository repository,
        BackupExecutionContext context,
        CancellationToken cancellationToken)
    {
        if (repository.Spec is FileSystemBackupRepositorySpec fs)
        {
            if (fs.Location != context.Location || fs.PlatformId != context.PlatformId)
                return Result.Failure<ResticRepositoryEnvironment>(new BadRequestError("Filesystem backup repository execution context does not match its configured location."));

            var secretMaterials = await LoadSecretMaterialsAsync(uow, [repository.PasswordSecretId], cancellationToken);
            var password = await ResolveSecretAsync(
                secretMaterials,
                repository.PasswordSecretId,
                "Repository password",
                new Dictionary<Guid, string>(),
                cancellationToken);
            if (!password.IsSuccess(out var passwordValue, out var passwordError))
                return Result.Failure<ResticRepositoryEnvironment>(passwordError!);

            if (context.Location == BackupExecutionLocation.Platform)
            {
                var platformPath = ResolveAndValidatePlatformRepositoryPath(fs.Path);
                if (!platformPath.IsSuccess(out var platformRepositoryPath, out var platformPathError))
                    return Result.Failure<ResticRepositoryEnvironment>(platformPathError!);

                return CreateEnvironment(
                    PlatformRepositoryMountPath,
                    passwordValue,
                    new Dictionary<string, string>(),
                    [],
                    context,
                    platformRepositoryHostPath: platformRepositoryPath,
                    repositoryRequiresNetwork: false);
            }

            var path = ResolveAndValidateRepositoryPath(fs.Path);
            if (!path.IsSuccess(out var repositoryPath, out var pathError))
                return Result.Failure<ResticRepositoryEnvironment>(pathError!);

            Directory.CreateDirectory(repositoryPath);
            return CreateEnvironment(
                repositoryPath,
                passwordValue,
                new Dictionary<string, string>(),
                [],
                context,
                platformRepositoryHostPath: null,
                repositoryRequiresNetwork: false);
        }

        if (repository.Spec is S3CompatibleBackupRepositorySpec s3)
        {
            var requiredSecretIds = new[]
            {
                repository.PasswordSecretId,
                s3.AccessKeySecretId,
                s3.SecretKeySecretId,
                s3.SessionTokenSecretId
            }
            .Where(static id => id.HasValue)
            .Select(static id => id!.Value)
            .ToArray();

            var secretMaterials = await LoadSecretMaterialsAsync(uow, requiredSecretIds, cancellationToken);
            var providerTokens = new Dictionary<Guid, string>();

            var password = await ResolveSecretAsync(secretMaterials, repository.PasswordSecretId, "Repository password", providerTokens, cancellationToken);
            if (!password.IsSuccess(out var passwordValue, out var passwordError))
                return Result.Failure<ResticRepositoryEnvironment>(passwordError!);

            var accessKey = await ResolveSecretAsync(secretMaterials, s3.AccessKeySecretId, "S3 access key", providerTokens, cancellationToken);
            if (!accessKey.IsSuccess(out var accessKeyValue, out var accessKeyError))
                return Result.Failure<ResticRepositoryEnvironment>(accessKeyError!);

            var secretKey = await ResolveSecretAsync(secretMaterials, s3.SecretKeySecretId, "S3 secret key", providerTokens, cancellationToken);
            if (!secretKey.IsSuccess(out var secretKeyValue, out var secretKeyError))
                return Result.Failure<ResticRepositoryEnvironment>(secretKeyError!);

            string? sessionTokenValue = null;
            if (s3.SessionTokenSecretId.HasValue)
            {
                var sessionToken = await ResolveSecretAsync(secretMaterials, s3.SessionTokenSecretId.Value, "S3 session token", providerTokens, cancellationToken);
                if (!sessionToken.IsSuccess(out sessionTokenValue, out var sessionTokenError))
                    return Result.Failure<ResticRepositoryEnvironment>(sessionTokenError!);
            }

            var env = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["AWS_ACCESS_KEY_ID"] = accessKeyValue,
                ["AWS_SECRET_ACCESS_KEY"] = secretKeyValue
            };

            if (!string.IsNullOrWhiteSpace(sessionTokenValue))
                env["AWS_SESSION_TOKEN"] = sessionTokenValue;

            if (!string.IsNullOrWhiteSpace(s3.Region))
                env["AWS_DEFAULT_REGION"] = s3.Region;

            var commonArgs = s3.BucketLookup == S3BucketLookup.Auto
                ? Array.Empty<string>()
                : ["-o", $"s3.bucket-lookup={s3.BucketLookup.ToString().ToLowerInvariant()}"];

            return CreateEnvironment(
                BuildS3RepositoryUri(s3),
                passwordValue,
                env,
                [accessKeyValue, secretKeyValue, sessionTokenValue ?? string.Empty],
                context,
                platformRepositoryHostPath: null,
                repositoryRequiresNetwork: true,
                commonArgs);
        }

        return Result.Failure<ResticRepositoryEnvironment>(new BadRequestError("Unsupported backup repository type."));
    }

    private Result<string> ResolveAndValidateRepositoryPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path is required."));

        var allowedRoots = options.AllowedCorePaths
            .Where(static x => !string.IsNullOrWhiteSpace(x))
            .Select(static x => EnsureTrailingSeparator(Path.GetFullPath(x)))
            .ToArray();
        var fullPath = ResolveRepositoryPath(path.Trim(), allowedRoots);
        var normalized = EnsureTrailingSeparator(fullPath);
        var denied =
            RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                ? Array.Empty<string>()
                : ["/var/run/docker.sock", "/proc", "/sys", "/dev"];

        if (denied.Any(deniedPath => normalized.StartsWith(EnsureTrailingSeparator(deniedPath), StringComparison.Ordinal)))
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path points to a protected system location."));

        if (allowedRoots.Length > 0 && !IsPathAllowed(normalized, allowedRoots))
        {
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path is outside the configured allowed backup paths."));
        }

        return fullPath;
    }

    private static string ResolveRepositoryPath(string path, IReadOnlyList<string> allowedRoots)
    {
        var fullPath = Path.GetFullPath(path);
        if (Path.IsPathFullyQualified(path) || allowedRoots.Count == 0 || IsPathAllowed(EnsureTrailingSeparator(fullPath), allowedRoots))
            return fullPath;

        return Path.GetFullPath(Path.Combine(allowedRoots[0], path));
    }

    private static bool IsPathAllowed(string normalizedPath, IReadOnlyCollection<string> allowedRoots)
    {
        var comparison = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? StringComparison.OrdinalIgnoreCase
            : StringComparison.Ordinal;
        return allowedRoots.Any(root => normalizedPath.StartsWith(root, comparison));
    }

    private static Result<string> ResolveAndValidatePlatformRepositoryPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path is required."));

        var normalized = path.Trim();
        if (normalized.Contains('\0'))
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path is invalid."));

        if (!IsAbsolutePlatformPath(normalized))
            return Result.Failure<string>(new BadRequestError("Platform filesystem backup repository path must be an absolute host path."));

        var pathForChecks = normalized.Replace('\\', '/').TrimEnd('/') + "/";
        var denied = new[] { "/var/run/docker.sock/", "/proc/", "/sys/", "/dev/" };
        if (denied.Any(deniedPath => pathForChecks.StartsWith(deniedPath, StringComparison.Ordinal)))
            return Result.Failure<string>(new BadRequestError("Filesystem backup repository path points to a protected system location."));

        return normalized;
    }

    private static bool IsAbsolutePlatformPath(string path)
        => path.StartsWith("/", StringComparison.Ordinal)
           || (path.Length >= 3
               && char.IsLetter(path[0])
               && path[1] == ':'
               && (path[2] == '\\' || path[2] == '/'));

    private static async Task<IReadOnlyDictionary<Guid, SecretResolutionMaterial>> LoadSecretMaterialsAsync(
        IUnitOfWork uow,
        IReadOnlyCollection<Guid> secretIds,
        CancellationToken cancellationToken)
    {
        var materials = await uow.SecretDefinitions.GetResolutionMaterialsAsync(secretIds, cancellationToken);
        return materials.ToDictionary(static x => x.Definition.Id);
    }

    private async Task<Result<string>> ResolveSecretAsync(
        IReadOnlyDictionary<Guid, SecretResolutionMaterial> secrets,
        Guid secretId,
        string label,
        IDictionary<Guid, string> providerTokens,
        CancellationToken cancellationToken)
    {
        if (!secrets.TryGetValue(secretId, out var material))
            return Result.Failure<string>(new BadRequestError($"{label} secret is not available."));

        var secret = material.Definition;
        if (secret.ProviderType == SecretProviderType.InternalEncrypted)
        {
            if (material.InternalValue is null)
                return Result.Failure<string>(new BadRequestError($"{label} secret is not available."));

            return Unprotect(material.InternalValue.EncryptedValue, $"{label} secret could not be decrypted.");
        }

        if (secret.ProviderId is null)
            return Result.Failure<string>(new BadRequestError($"{label} secret does not reference a provider."));

        var provider = material.Provider;
        if (provider is null)
            return Result.Failure<string>(new BadRequestError($"{label} secret provider is not available."));

        if (!providerTokens.TryGetValue(provider.Id, out var tokenValue))
        {
            var token = Unprotect(provider.Configuration.ProtectedToken, $"{label} secret provider token could not be decrypted.");
            if (!token.IsSuccess(out tokenValue, out var tokenError))
                return Result.Failure<string>(tokenError!);

            providerTokens[provider.Id] = tokenValue;
        }

        var result = await externalSecretProviderClient.ResolveAsync(secret, provider, tokenValue, cancellationToken);
        return result.IsSuccess
            ? Result.Success(result.Value ?? string.Empty)
            : Result.Failure<string>(new BadRequestError(result.ErrorMessage ?? $"{label} secret could not be resolved."));
    }

    private Result<string> Unprotect(string value, string message)
    {
        try
        {
            return secretValueProtector.Unprotect(value);
        }
        catch
        {
            return Result.Failure<string>(new BadRequestError(message));
        }
    }

    private ResticRepositoryEnvironment CreateEnvironment(
        string repository,
        string password,
        IReadOnlyDictionary<string, string> environment,
        IReadOnlyCollection<string> redactionValues,
        BackupExecutionContext context,
        string? platformRepositoryHostPath,
        bool repositoryRequiresNetwork,
        IReadOnlyList<string>? commonArguments = null)
    {
        var workDir = Path.GetFullPath(options.WorkingDirectory);
        var secretDir = Path.Combine(workDir, "secrets");
        Directory.CreateDirectory(secretDir);

        var passwordPath = Path.Combine(secretDir, $"{Guid.NewGuid():N}.password");
        File.WriteAllText(passwordPath, password);
        SetOwnerOnlyPermissions(passwordPath);

        var env = new Dictionary<string, string>(environment, StringComparer.Ordinal)
        {
            ["RESTIC_REPOSITORY"] = repository,
            ["RESTIC_PASSWORD_FILE"] = passwordPath
        };

        var remoteEnv = new Dictionary<string, string>(environment, StringComparer.Ordinal)
        {
            ["RESTIC_REPOSITORY"] = repository,
            ["RESTIC_PASSWORD"] = password
        };

        return new ResticRepositoryEnvironment(
            workDir,
            commonArguments ?? [],
            env,
            remoteEnv,
            [password, repository, .. redactionValues],
            passwordPath,
            context,
            platformRepositoryHostPath,
            repositoryRequiresNetwork);
    }

    private static string BuildS3RepositoryUri(S3CompatibleBackupRepositorySpec spec)
    {
        var endpoint = spec.Endpoint.ToString().TrimEnd('/');
        var prefix = string.IsNullOrWhiteSpace(spec.Prefix) ? string.Empty : "/" + spec.Prefix.Trim('/');
        return $"s3:{endpoint}/{spec.Bucket.Trim('/')}{prefix}";
    }

    private static string EnsureTrailingSeparator(string path)
        => path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + Path.DirectorySeparatorChar;

    private static void SetOwnerOnlyPermissions(string filePath)
    {
        if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            File.SetUnixFileMode(filePath, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    }

    private const string PlatformRepositoryMountPath = "/repository";
}

internal sealed record LoadedBackupRepository(BackupRepository Repository, ResticRepositoryEnvironment Environment);

internal sealed record ResticOperationResult(int ExitCode, string StdOut, string StdErr);

internal sealed class ResticRepositoryEnvironment(
    string workingDirectory,
    IReadOnlyList<string> commonArguments,
    IReadOnlyDictionary<string, string> environment,
    IReadOnlyDictionary<string, string> remoteEnvironment,
    IReadOnlyCollection<string> redactionValues,
    string passwordFilePath,
    BackupExecutionContext context,
    string? platformRepositoryHostPath,
    bool repositoryRequiresNetwork) : IAsyncDisposable
{
    public string WorkingDirectory { get; } = workingDirectory;
    public IReadOnlyList<string> CommonArguments { get; } = commonArguments;
    public IReadOnlyDictionary<string, string> Environment { get; } = environment;
    public IReadOnlyDictionary<string, string> RemoteEnvironment { get; } = remoteEnvironment;
    public IReadOnlyCollection<string> RedactionValues { get; } = redactionValues;
    public BackupExecutionContext Context { get; } = context;
    public string? PlatformRepositoryHostPath { get; } = platformRepositoryHostPath;
    public bool RepositoryRequiresNetwork { get; } = repositoryRequiresNetwork;

    public ValueTask DisposeAsync()
    {
        try
        {
            File.Delete(passwordFilePath);
        }
        catch
        {
        }

        return ValueTask.CompletedTask;
    }
}
