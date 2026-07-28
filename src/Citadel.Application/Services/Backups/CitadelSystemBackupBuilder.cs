using Application.Configs;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using NSec.Cryptography;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Application.Services.Backups;

internal interface ICitadelSystemBackupBuilder
{
    Task<Result<CitadelBackupBundle>> BuildAsync(
        Guid runId,
        TimeSpan timeout,
        CancellationToken cancellationToken);

    int CleanupStaleBundles();
}

internal interface ICitadelRecoveryAssetProvider
{
    IReadOnlyList<CitadelRecoveryAsset> GetAssets();
}

internal enum RecoveryAssetOrigin
{
    File,
    ExternalConfiguration,
    Missing
}

internal sealed record CitadelRecoveryAsset(
    string Name,
    string RelativeArchivePath,
    RecoveryAssetOrigin Origin,
    bool Required,
    string? SourcePath,
    bool IsDirectory);

internal sealed class CitadelRecoveryAssetProvider(
    IConfiguration configuration,
    IOptions<BackupOptions> backupOptions) : ICitadelRecoveryAssetProvider
{
    private readonly BackupOptions options = backupOptions.Value;

    public IReadOnlyList<CitadelRecoveryAsset> GetAssets()
    {
        var dataRoot = Path.GetFullPath(options.CoreDataPath);
        return
        [
            CreateFileAsset(
                "jwtsecret",
                "recovery/jwtsecret",
                Path.Combine(dataRoot, "jwtsecret"),
                configuration["Jwt:Key"]),
            CreateFileAsset(
                "secret-encryption-key",
                "recovery/secret-encryption-key",
                Path.Combine(dataRoot, "secret-encryption-key"),
                configuration["Secrets:EncryptionKey"]),
            CreateFileAsset(
                "keys/id_ed25519",
                "recovery/keys/id_ed25519",
                Path.Combine(dataRoot, "keys", "id_ed25519"),
                externalValue: null),
            CreateFileAsset(
                "keys/id_ed25519.pub",
                "recovery/keys/id_ed25519.pub",
                Path.Combine(dataRoot, "keys", "id_ed25519.pub"),
                externalValue: null),
            CreateDirectoryAsset(
                "keys/dataprotection",
                "recovery/keys/dataprotection",
                Path.Combine(dataRoot, "keys", "dataprotection"))
        ];
    }

    private static CitadelRecoveryAsset CreateFileAsset(
        string name,
        string archivePath,
        string sourcePath,
        string? externalValue)
    {
        if (!string.IsNullOrWhiteSpace(externalValue))
            return new CitadelRecoveryAsset(
                name,
                archivePath,
                RecoveryAssetOrigin.ExternalConfiguration,
                Required: true,
                SourcePath: null,
                IsDirectory: false);

        return new CitadelRecoveryAsset(
            name,
            archivePath,
            File.Exists(sourcePath) ? RecoveryAssetOrigin.File : RecoveryAssetOrigin.Missing,
            Required: true,
            sourcePath,
            IsDirectory: false);
    }

    private static CitadelRecoveryAsset CreateDirectoryAsset(
        string name,
        string archivePath,
        string sourcePath)
        => new(
            name,
            archivePath,
            Directory.Exists(sourcePath) ? RecoveryAssetOrigin.File : RecoveryAssetOrigin.Missing,
            Required: false,
            sourcePath,
            IsDirectory: true);
}

internal sealed class CitadelSystemBackupBuilder(
    IServiceScopeFactory scopeFactory,
    IPostgresDumpRunner postgresDumpRunner,
    ICitadelRecoveryAssetProvider recoveryAssetProvider,
    IConfiguration configuration,
    IOptions<BackupOptions> backupOptions,
    TimeProvider timeProvider,
    ILogger<CitadelSystemBackupBuilder> logger) : ICitadelSystemBackupBuilder
{
    private const int ManifestFormatVersion = 1;
    private readonly BackupOptions options = backupOptions.Value;

    public async Task<Result<CitadelBackupBundle>> BuildAsync(
        Guid runId,
        TimeSpan timeout,
        CancellationToken cancellationToken)
    {
        var stagingPath = CreateStagingPath(runId);
        try
        {
            RecreatePrivateDirectory(stagingPath);
            var databasePath = Path.Combine(stagingPath, "database");
            Directory.CreateDirectory(databasePath);
            SetPrivateDirectoryMode(databasePath);

            var connectionString = configuration.GetConnectionString("Postgres");
            if (string.IsNullOrWhiteSpace(connectionString))
                throw new CitadelBackupBundleException("The PostgreSQL connection string is not configured.");

            var instanceId = await GetInstanceIdAsync(cancellationToken);
            var dumpPath = Path.Combine(databasePath, "citadel.dump");
            var dump = await postgresDumpRunner.CreateDumpAsync(
                new PostgresDumpCommand(
                    options.PostgresDumpPath,
                    connectionString,
                    dumpPath,
                    stagingPath,
                    timeout,
                    Math.Max(1024, options.MaxLogLineBytes)),
                cancellationToken);
            if (!dump.Succeeded)
                throw new CitadelBackupBundleException(
                    dump.ErrorMessage ?? $"pg_dump exited with code {dump.ExitCode}.");

            if (!File.Exists(dumpPath))
                throw new CitadelBackupBundleException("pg_dump completed without creating the database archive.");

            SetPrivateFileMode(dumpPath);
            await ValidatePostgresDumpAsync(dumpPath, cancellationToken);
            var checksums = new SortedDictionary<string, string>(StringComparer.Ordinal);
            var dumpFile = await DescribeFileAsync(stagingPath, dumpPath, checksums, cancellationToken);
            var warnings = new List<BackupRunWarning>();
            var assets = await CopyRecoveryAssetsAsync(
                stagingPath,
                checksums,
                warnings,
                cancellationToken);
            ValidateAgentKeyPair(stagingPath, assets);

            var manifest = new CitadelBackupManifest(
                ManifestFormatVersion,
                "citadel",
                ApplicationVersion.CoreInformationalVersion,
                Constants.CompatibilityVersion,
                instanceId,
                timeProvider.GetUtcNow(),
                new CitadelDatabaseManifest(
                    "PostgreSQL",
                    dumpFile.RelativePath,
                    dump.ServerVersion ?? "unknown",
                    dumpFile.Sha256),
                assets);

            var manifestPath = Path.Combine(stagingPath, "manifest.json");
            await WriteJsonAsync(
                manifestPath,
                manifest,
                CitadelBackupJsonContext.Default.CitadelBackupManifest,
                cancellationToken);
            SetPrivateFileMode(manifestPath);
            await DescribeFileAsync(stagingPath, manifestPath, checksums, cancellationToken);

            var checksumsPath = Path.Combine(stagingPath, "checksums.json");
            await WriteJsonAsync(
                checksumsPath,
                checksums,
                CitadelBackupJsonContext.Default.SortedDictionaryStringString,
                cancellationToken);
            SetPrivateFileMode(checksumsPath);

            return Result.Success(new CitadelBackupBundle(stagingPath, warnings));
        }
        catch (OperationCanceledException)
        {
            TryDeleteStagingDirectory(stagingPath);
            throw;
        }
        catch (CitadelBackupBundleException ex)
        {
            TryDeleteStagingDirectory(stagingPath);
            return Result.Failure<CitadelBackupBundle>(new InternalServerError(ex.Message));
        }
        catch (Exception ex)
        {
            TryDeleteStagingDirectory(stagingPath);
            logger.LogError(ex, "Citadel recovery bundle creation failed for backup run {RunId}.", runId);
            return Result.Failure<CitadelBackupBundle>(
                new InternalServerError("Citadel could not create the recovery bundle."));
        }
    }

    public int CleanupStaleBundles()
    {
        var removed = 0;
        try
        {
            var root = GetStagingRoot();
            if (!Directory.Exists(root))
                return 0;

            var rootInfo = new DirectoryInfo(root);
            if ((rootInfo.Attributes & FileAttributes.ReparsePoint) != 0)
            {
                logger.LogError(
                    "Citadel backup staging root {StagingRoot} is a symbolic link. Stale bundle cleanup was skipped.",
                    root);
                return 0;
            }

            foreach (var directory in rootInfo.EnumerateDirectories())
            {
                try
                {
                    DeleteDirectoryEntry(directory);
                    removed++;
                }
                catch (Exception ex)
                {
                    logger.LogError(
                        ex,
                        "Citadel could not delete stale recovery bundle staging directory {StagingPath}.",
                        directory.FullName);
                }
            }
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Citadel could not inspect the recovery bundle staging directory under {WorkingDirectory}.",
                options.WorkingDirectory);
        }

        return removed;
    }

    private async Task<Guid> GetInstanceIdAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(
            Guid.CreateVersion7(),
            timeProvider.GetUtcNow(),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return identity.InstanceId;
    }

    private async Task<IReadOnlyList<RecoveryAssetManifestEntry>> CopyRecoveryAssetsAsync(
        string stagingPath,
        IDictionary<string, string> checksums,
        ICollection<BackupRunWarning> warnings,
        CancellationToken cancellationToken)
    {
        var entries = new List<RecoveryAssetManifestEntry>();
        foreach (var asset in recoveryAssetProvider.GetAssets())
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (asset.Origin == RecoveryAssetOrigin.Missing)
            {
                if (asset.Required)
                    throw new CitadelBackupBundleException(
                        $"Required recovery asset \"{asset.Name}\" is missing.");

                entries.Add(new RecoveryAssetManifestEntry(
                    asset.Name,
                    asset.RelativeArchivePath,
                    asset.Origin,
                    asset.Required,
                    Sha256: null,
                    SizeBytes: null));
                warnings.Add(new BackupRunWarning(
                    "backup.recovery_asset_missing",
                    $"Optional recovery asset \"{asset.Name}\" is missing."));
                continue;
            }

            if (asset.Origin == RecoveryAssetOrigin.ExternalConfiguration)
            {
                entries.Add(new RecoveryAssetManifestEntry(
                    asset.Name,
                    asset.RelativeArchivePath,
                    asset.Origin,
                    asset.Required,
                    Sha256: null,
                    SizeBytes: null));
                continue;
            }

            var sourcePath = asset.SourcePath
                ?? throw new CitadelBackupBundleException(
                    $"Recovery asset \"{asset.Name}\" has no source path.");
            var destinationPath = ResolveArchivePath(stagingPath, asset.RelativeArchivePath);
            if (asset.IsDirectory)
            {
                var files = await CopyDirectoryAsync(
                    stagingPath,
                    sourcePath,
                    destinationPath,
                    asset,
                    checksums,
                    cancellationToken);
                entries.AddRange(files);
            }
            else
            {
                Directory.CreateDirectory(Path.GetDirectoryName(destinationPath)!);
                await CopyFileAsync(sourcePath, destinationPath, cancellationToken);
                var file = await DescribeFileAsync(
                    stagingPath,
                    destinationPath,
                    checksums,
                    cancellationToken);
                entries.Add(new RecoveryAssetManifestEntry(
                    asset.Name,
                    file.RelativePath,
                    asset.Origin,
                    asset.Required,
                    file.Sha256,
                    file.SizeBytes));
            }
        }

        return entries;
    }

    private static async Task<IReadOnlyList<RecoveryAssetManifestEntry>> CopyDirectoryAsync(
        string stagingPath,
        string sourcePath,
        string destinationPath,
        CitadelRecoveryAsset asset,
        IDictionary<string, string> checksums,
        CancellationToken cancellationToken)
    {
        var sourceRoot = new DirectoryInfo(Path.GetFullPath(sourcePath));
        if ((sourceRoot.Attributes & FileAttributes.ReparsePoint) != 0)
            throw new CitadelBackupBundleException(
                $"Recovery asset directory \"{sourceRoot.Name}\" cannot be a symbolic link.");

        Directory.CreateDirectory(destinationPath);
        SetPrivateDirectoryMode(destinationPath);
        var files = new List<RecoveryAssetManifestEntry>();
        var pendingDirectories = new Stack<DirectoryInfo>();
        pendingDirectories.Push(sourceRoot);
        while (pendingDirectories.TryPop(out var sourceDirectory))
        {
            foreach (var childDirectory in sourceDirectory
                         .EnumerateDirectories()
                         .OrderBy(static directory => directory.FullName, StringComparer.Ordinal))
            {
                if ((childDirectory.Attributes & FileAttributes.ReparsePoint) != 0)
                    throw new CitadelBackupBundleException(
                        $"Recovery asset directory \"{childDirectory.Name}\" cannot be a symbolic link.");

                var relativeDirectory = Path.GetRelativePath(sourceRoot.FullName, childDirectory.FullName);
                var destinationDirectory = Path.Combine(destinationPath, relativeDirectory);
                Directory.CreateDirectory(destinationDirectory);
                SetPrivateDirectoryMode(destinationDirectory);
                pendingDirectories.Push(childDirectory);
            }

            foreach (var sourceFile in sourceDirectory
                         .EnumerateFiles()
                         .OrderBy(static file => file.FullName, StringComparer.Ordinal))
            {
                cancellationToken.ThrowIfCancellationRequested();
                if ((sourceFile.Attributes & FileAttributes.ReparsePoint) != 0)
                    throw new CitadelBackupBundleException(
                        $"Recovery asset file \"{sourceFile.Name}\" cannot be a symbolic link.");

                var relativePath = Path.GetRelativePath(sourceRoot.FullName, sourceFile.FullName);
                var destinationFile = Path.Combine(destinationPath, relativePath);
                var destinationDirectory = Path.GetDirectoryName(destinationFile)!;
                Directory.CreateDirectory(destinationDirectory);
                SetPrivateDirectoryMode(destinationDirectory);
                await CopyFileAsync(sourceFile.FullName, destinationFile, cancellationToken);
                var file = await DescribeFileAsync(
                    stagingPath,
                    destinationFile,
                    checksums,
                    cancellationToken);
                var archiveRelativePath = ToArchivePath(relativePath);
                files.Add(new RecoveryAssetManifestEntry(
                    $"{asset.Name}/{archiveRelativePath}",
                    file.RelativePath,
                    RecoveryAssetOrigin.File,
                    asset.Required,
                    file.Sha256,
                    file.SizeBytes));
            }
        }

        return files;
    }

    private static async Task CopyFileAsync(
        string sourcePath,
        string destinationPath,
        CancellationToken cancellationToken)
    {
        await using var source = new FileStream(
            sourcePath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            bufferSize: 64 * 1024,
            FileOptions.Asynchronous | FileOptions.SequentialScan);
        await using var destination = new FileStream(
            destinationPath,
            FileMode.CreateNew,
            FileAccess.Write,
            FileShare.None,
            bufferSize: 64 * 1024,
            FileOptions.Asynchronous | FileOptions.SequentialScan);
        await source.CopyToAsync(destination, cancellationToken);
        await destination.FlushAsync(cancellationToken);
        SetPrivateFileMode(destinationPath);
    }

    private static void ValidateAgentKeyPair(
        string stagingPath,
        IReadOnlyCollection<RecoveryAssetManifestEntry> assets)
    {
        var privateKeyEntry = assets.SingleOrDefault(static asset =>
            asset.Origin == RecoveryAssetOrigin.File
            && string.Equals(asset.Name, "keys/id_ed25519", StringComparison.Ordinal));
        var publicKeyEntry = assets.SingleOrDefault(static asset =>
            asset.Origin == RecoveryAssetOrigin.File
            && string.Equals(asset.Name, "keys/id_ed25519.pub", StringComparison.Ordinal));
        if (privateKeyEntry is null || publicKeyEntry is null)
            throw new CitadelBackupBundleException("The Core-to-Agent key pair is incomplete.");

        try
        {
            var privateKeyBytes = File.ReadAllBytes(
                ResolveArchivePath(stagingPath, privateKeyEntry.ArchivePath));
            var publicKeyBytes = File.ReadAllBytes(
                ResolveArchivePath(stagingPath, publicKeyEntry.ArchivePath));
            using var privateKey = Key.Import(
                SignatureAlgorithm.Ed25519,
                privateKeyBytes,
                KeyBlobFormat.RawPrivateKey);
            var expectedPublicKey = privateKey.PublicKey.Export(KeyBlobFormat.RawPublicKey);
            if (!CryptographicOperations.FixedTimeEquals(expectedPublicKey, publicKeyBytes))
            {
                throw new CitadelBackupBundleException(
                    "The Core-to-Agent key pair changed while the backup was being created. Retry the backup.");
            }
        }
        catch (CitadelBackupBundleException)
        {
            throw;
        }
        catch (Exception)
        {
            throw new CitadelBackupBundleException("The Core-to-Agent key pair is invalid.");
        }
    }

    private static async Task<BackupFileManifestEntry> DescribeFileAsync(
        string stagingPath,
        string filePath,
        IDictionary<string, string> checksums,
        CancellationToken cancellationToken)
    {
        await using var stream = new FileStream(
            filePath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            bufferSize: 64 * 1024,
            FileOptions.Asynchronous | FileOptions.SequentialScan);
        var hash = Convert.ToHexString(await SHA256.HashDataAsync(stream, cancellationToken))
            .ToLowerInvariant();
        var relativePath = ToArchivePath(Path.GetRelativePath(stagingPath, filePath));
        checksums.Add(relativePath, hash);
        return new BackupFileManifestEntry(relativePath, hash, stream.Length);
    }

    private static async Task ValidatePostgresDumpAsync(
        string dumpPath,
        CancellationToken cancellationToken)
    {
        var signature = new byte[5];
        await using var stream = new FileStream(
            dumpPath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            bufferSize: signature.Length,
            FileOptions.Asynchronous | FileOptions.SequentialScan);
        var bytesRead = await stream.ReadAtLeastAsync(
            signature,
            signature.Length,
            throwOnEndOfStream: false,
            cancellationToken);
        if (bytesRead != signature.Length
            || !"PGDMP"u8.SequenceEqual(signature))
        {
            throw new CitadelBackupBundleException(
                "pg_dump did not create a valid PostgreSQL custom-format archive.");
        }
    }

    private string CreateStagingPath(Guid runId)
    {
        var root = GetStagingRoot();
        EnsurePrivateStagingRoot(root);
        var stagingPath = Path.Combine(root, runId.ToString("N"));
        var normalizedRoot = EnsureTrailingSeparator(root);
        var normalizedStaging = Path.GetFullPath(stagingPath);
        if (!normalizedStaging.StartsWith(
                normalizedRoot,
                OperatingSystem.IsWindows()
                    ? StringComparison.OrdinalIgnoreCase
                    : StringComparison.Ordinal))
        {
            throw new InvalidOperationException("Citadel backup staging path is outside the backup working directory.");
        }

        return normalizedStaging;
    }

    private string GetStagingRoot()
    {
        var workingDirectory = Path.GetFullPath(options.WorkingDirectory);
        var root = Path.GetFullPath(Path.Combine(workingDirectory, "citadel-system"));
        if (!root.StartsWith(
                EnsureTrailingSeparator(workingDirectory),
                OperatingSystem.IsWindows()
                    ? StringComparison.OrdinalIgnoreCase
                    : StringComparison.Ordinal))
        {
            throw new InvalidOperationException(
                "Citadel backup staging root is outside the backup working directory.");
        }

        return root;
    }

    private static void EnsurePrivateStagingRoot(string root)
    {
        Directory.CreateDirectory(root);
        var rootInfo = new DirectoryInfo(root);
        if ((rootInfo.Attributes & FileAttributes.ReparsePoint) != 0)
        {
            throw new CitadelBackupBundleException(
                "Citadel backup staging root cannot be a symbolic link.");
        }

        SetPrivateDirectoryMode(root);
    }

    private static string ResolveArchivePath(string stagingPath, string relativeArchivePath)
    {
        var resolved = Path.GetFullPath(Path.Combine(
            stagingPath,
            relativeArchivePath.Replace('/', Path.DirectorySeparatorChar)));
        if (!resolved.StartsWith(
                EnsureTrailingSeparator(Path.GetFullPath(stagingPath)),
                OperatingSystem.IsWindows()
                    ? StringComparison.OrdinalIgnoreCase
                    : StringComparison.Ordinal))
        {
            throw new CitadelBackupBundleException("Recovery asset archive path is invalid.");
        }

        return resolved;
    }

    private static async Task WriteJsonAsync<T>(
        string path,
        T value,
        System.Text.Json.Serialization.Metadata.JsonTypeInfo<T> typeInfo,
        CancellationToken cancellationToken)
    {
        await using var stream = new FileStream(
            path,
            FileMode.CreateNew,
            FileAccess.Write,
            FileShare.None,
            bufferSize: 16 * 1024,
            FileOptions.Asynchronous);
        await JsonSerializer.SerializeAsync(stream, value, typeInfo, cancellationToken);
        await stream.FlushAsync(cancellationToken);
    }

    private static void RecreatePrivateDirectory(string path)
    {
        if (Directory.Exists(path))
            DeleteDirectoryEntry(new DirectoryInfo(path));
        Directory.CreateDirectory(path);
        SetPrivateDirectoryMode(path);
    }

    private static void DeleteDirectoryEntry(DirectoryInfo directory)
        => directory.Delete(
            recursive: (directory.Attributes & FileAttributes.ReparsePoint) == 0);

    private void TryDeleteStagingDirectory(string path)
    {
        try
        {
            if (Directory.Exists(path))
                Directory.Delete(path, recursive: true);
        }
        catch (Exception ex)
        {
            logger.LogError(
                ex,
                "Citadel could not delete failed recovery bundle staging directory {StagingPath}.",
                path);
        }
    }

    private static void SetPrivateDirectoryMode(string path)
    {
        if (!OperatingSystem.IsWindows())
        {
            File.SetUnixFileMode(
                path,
                UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute);
        }
    }

    private static void SetPrivateFileMode(string path)
    {
        if (!OperatingSystem.IsWindows())
            File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    }

    private static string EnsureTrailingSeparator(string path)
        => path.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
           + Path.DirectorySeparatorChar;

    private static string ToArchivePath(string path)
        => path.Replace(Path.DirectorySeparatorChar, '/');
}

internal sealed class CitadelBackupBundle(
    string rootPath,
    IReadOnlyList<BackupRunWarning> warnings) : IAsyncDisposable
{
    public string RootPath { get; } = rootPath;
    public IReadOnlyList<BackupRunWarning> Warnings { get; } = warnings;

    public ValueTask DisposeAsync()
    {
        if (Directory.Exists(RootPath))
            Directory.Delete(RootPath, recursive: true);

        return ValueTask.CompletedTask;
    }
}

internal sealed class CitadelBackupBundleException(string message) : Exception(message);

internal sealed record CitadelBackupManifest(
    int FormatVersion,
    string Product,
    string CoreInformationalVersion,
    string CompatibilityVersion,
    Guid InstanceId,
    DateTimeOffset CreatedAt,
    CitadelDatabaseManifest Database,
    IReadOnlyList<RecoveryAssetManifestEntry> Assets);

internal sealed record CitadelDatabaseManifest(
    string Engine,
    string DumpFile,
    string ServerVersion,
    string Sha256);

internal sealed record RecoveryAssetManifestEntry(
    string Name,
    string ArchivePath,
    RecoveryAssetOrigin Origin,
    bool Required,
    string? Sha256,
    long? SizeBytes);

internal sealed record BackupFileManifestEntry(
    string RelativePath,
    string Sha256,
    long SizeBytes);

[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    WriteIndented = true,
    Converters = [typeof(JsonStringEnumConverter<RecoveryAssetOrigin>)])]
[JsonSerializable(typeof(CitadelBackupManifest))]
[JsonSerializable(typeof(SortedDictionary<string, string>))]
internal partial class CitadelBackupJsonContext : JsonSerializerContext;
