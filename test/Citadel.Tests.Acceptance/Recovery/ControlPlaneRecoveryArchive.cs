using Hosting.Common;
using Npgsql;
using System.IO.Compression;
using System.Reflection;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;
using Tests.Acceptance.Infrastructure;

namespace Tests.Acceptance.Recovery;

internal static class ControlPlaneRecoveryArchive
{
    private const int FormatVersion = 1;
    private const string Product = "citadel";
    private const string ManifestPath = "manifest.json";
    private const string DatabasePath = "database/citadel.dump";
    private const string RecoveryPrefix = "recovery/";

    private static readonly string[] RequiredRecoveryAssets =
    [
        "jwtsecret",
        "secret-encryption-key",
        "keys/id_ed25519",
        "keys/id_ed25519.pub"
    ];

    public static async Task CreateAsync(
        AcceptancePostgresFixture postgres,
        string connectionString,
        string dataDirectory,
        string archivePath,
        CancellationToken cancellationToken)
    {
        var assets = DiscoverAssets(dataDirectory);
        var dump = await postgres.DumpDatabaseAsync(connectionString, cancellationToken);
        var databaseHash = Sha256(dump);
        var (instanceId, serverVersion) = await ReadDatabaseMetadataAsync(
            connectionString,
            cancellationToken);

        var archiveDirectory = Path.GetDirectoryName(archivePath);
        if (!string.IsNullOrWhiteSpace(archiveDirectory))
            Directory.CreateDirectory(archiveDirectory);

        await using var archiveStream = new FileStream(
            archivePath,
            FileMode.CreateNew,
            FileAccess.ReadWrite,
            FileShare.None,
            bufferSize: 81920,
            useAsync: true);
        using var archive = new ZipArchive(
            archiveStream,
            ZipArchiveMode.Create,
            leaveOpen: true);

        await WriteEntryAsync(archive, DatabasePath, dump, cancellationToken);

        var assetEntries = new List<RecoveryAssetManifestEntry>(assets.Count);
        foreach (var asset in assets)
        {
            var content = await File.ReadAllBytesAsync(asset.SourcePath, cancellationToken);
            var archiveEntry = $"{RecoveryPrefix}{NormalizeArchivePath(asset.RelativePath)}";
            await WriteEntryAsync(archive, archiveEntry, content, cancellationToken);
            assetEntries.Add(new RecoveryAssetManifestEntry(
                asset.RelativePath,
                archiveEntry,
                asset.Required,
                Sha256(content)));
        }

        var assembly = typeof(WebApi.Routes.PublicEndpoints).Assembly;
        var manifest = new ControlPlaneRecoveryManifest(
            FormatVersion,
            Product,
            assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
                ?? assembly.GetName().Version?.ToString()
                ?? "unknown",
            Constants.CompatibilityVersion,
            instanceId,
            DateTimeOffset.UtcNow,
            new DatabaseManifest(
                "PostgreSQL",
                DatabasePath,
                serverVersion,
                databaseHash),
            assetEntries);
        var manifestBytes = JsonSerializer.SerializeToUtf8Bytes(
            manifest,
            RecoveryJsonContext.Default.ControlPlaneRecoveryManifest);
        await WriteEntryAsync(archive, ManifestPath, manifestBytes, cancellationToken);
    }

    public static async Task RestoreAsync(
        AcceptancePostgresFixture postgres,
        string connectionString,
        string archivePath,
        string targetDataDirectory,
        CancellationToken cancellationToken)
    {
        if (Directory.Exists(targetDataDirectory)
            && Directory.EnumerateFileSystemEntries(targetDataDirectory).Any())
        {
            throw new InvalidOperationException(
                "Control-plane recovery data directory must be empty.");
        }

        if (!await AcceptancePostgresFixture.IsDatabaseEmptyAsync(
                connectionString,
                cancellationToken))
        {
            throw new InvalidOperationException(
                "Control-plane recovery must target an empty PostgreSQL database.");
        }

        await using var archiveStream = File.OpenRead(archivePath);
        using var archive = new ZipArchive(
            archiveStream,
            ZipArchiveMode.Read,
            leaveOpen: true);
        var entries = BuildEntryIndex(archive);
        var manifestBytes = await ReadRequiredEntryAsync(
            entries,
            ManifestPath,
            cancellationToken);
        var manifest = JsonSerializer.Deserialize(
            manifestBytes,
            RecoveryJsonContext.Default.ControlPlaneRecoveryManifest)
            ?? throw new InvalidDataException("Recovery manifest is invalid.");

        ValidateManifest(manifest);

        var dump = await ReadAndValidateAsync(
            entries,
            manifest.Database.DumpFile,
            manifest.Database.Sha256,
            cancellationToken);

        var assetContents = new Dictionary<RecoveryAssetManifestEntry, byte[]>();
        if (manifest.Assets
            .GroupBy(asset => asset.Name, StringComparer.Ordinal)
            .Any(group => group.Count() > 1))
        {
            throw new InvalidDataException(
                "Recovery manifest contains duplicate asset names.");
        }

        if (manifest.Assets
            .GroupBy(asset => asset.ArchivePath, StringComparer.Ordinal)
            .Any(group => group.Count() > 1))
        {
            throw new InvalidDataException(
                "Recovery manifest contains duplicate asset paths.");
        }

        foreach (var requiredAsset in RequiredRecoveryAssets)
        {
            if (!manifest.Assets.Any(asset =>
                    asset.Required
                    && string.Equals(
                        asset.Name,
                        requiredAsset,
                        StringComparison.Ordinal)))
            {
                throw new InvalidDataException(
                    $"Required recovery asset '{requiredAsset}' is missing.");
            }
        }

        foreach (var asset in manifest.Assets)
        {
            ValidateRelativePath(asset.Name, "Recovery asset");
            if (!asset.ArchivePath.StartsWith(
                    RecoveryPrefix,
                    StringComparison.Ordinal))
            {
                throw new InvalidDataException(
                    $"Recovery asset '{asset.Name}' has an invalid archive path.");
            }

            assetContents.Add(
                asset,
                await ReadAndValidateAsync(
                    entries,
                    asset.ArchivePath,
                    asset.Sha256,
                    cancellationToken));
        }

        var stagingDirectory =
            $"{targetDataDirectory}.restore-{Guid.NewGuid():N}";
        try
        {
            Directory.CreateDirectory(stagingDirectory);
            foreach (var (asset, content) in assetContents)
            {
                var destination = GetSafeDestination(
                    stagingDirectory,
                    asset.Name);
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                await File.WriteAllBytesAsync(
                    destination,
                    content,
                    cancellationToken);
            }

            await postgres.RestoreDatabaseAsync(
                connectionString,
                dump,
                cancellationToken);

            if (Directory.Exists(targetDataDirectory))
                Directory.Delete(targetDataDirectory);
            Directory.Move(stagingDirectory, targetDataDirectory);
        }
        finally
        {
            if (Directory.Exists(stagingDirectory))
                Directory.Delete(stagingDirectory, recursive: true);
        }
    }

    private static IReadOnlyList<RecoveryAsset> DiscoverAssets(
        string dataDirectory)
    {
        var assets = new List<RecoveryAsset>();
        foreach (var relativePath in RequiredRecoveryAssets)
        {
            var sourcePath = GetSafeDestination(
                dataDirectory,
                relativePath);
            if (!File.Exists(sourcePath))
            {
                throw new InvalidDataException(
                    $"Required recovery asset '{relativePath}' is missing.");
            }

            assets.Add(new RecoveryAsset(
                relativePath,
                sourcePath,
                Required: true));
        }

        var dataProtectionDirectory = Path.Combine(
            dataDirectory,
            "keys",
            "dataprotection");
        if (Directory.Exists(dataProtectionDirectory))
        {
            assets.AddRange(
                Directory.EnumerateFiles(
                        dataProtectionDirectory,
                        "*",
                        SearchOption.AllDirectories)
                    .Order(StringComparer.Ordinal)
                    .Select(path => new RecoveryAsset(
                        Path.GetRelativePath(dataDirectory, path),
                        path,
                        Required: false)));
        }

        return assets;
    }

    private static Dictionary<string, ZipArchiveEntry> BuildEntryIndex(
        ZipArchive archive)
    {
        var entries = new Dictionary<string, ZipArchiveEntry>(
            StringComparer.Ordinal);
        foreach (var entry in archive.Entries)
        {
            if (!entries.TryAdd(entry.FullName, entry))
            {
                throw new InvalidDataException(
                    $"Recovery archive contains duplicate entry '{entry.FullName}'.");
            }
        }

        return entries;
    }

    private static void ValidateManifest(
        ControlPlaneRecoveryManifest manifest)
    {
        if (manifest.FormatVersion != FormatVersion)
        {
            throw new InvalidDataException(
                $"Unsupported recovery format version {manifest.FormatVersion}.");
        }

        if (!string.Equals(manifest.Product, Product, StringComparison.Ordinal))
            throw new InvalidDataException("Recovery archive is not a Citadel backup.");

        if (manifest.InstanceId == Guid.Empty)
            throw new InvalidDataException("Recovery archive instance identity is invalid.");

        if (!string.Equals(
                manifest.Database.Engine,
                "PostgreSQL",
                StringComparison.Ordinal))
        {
            throw new InvalidDataException(
                "Recovery archive database engine is not supported.");
        }

        if (!string.Equals(
                manifest.Database.DumpFile,
                DatabasePath,
                StringComparison.Ordinal))
        {
            throw new InvalidDataException(
                "Recovery archive database path is invalid.");
        }
    }

    private static async Task<byte[]> ReadAndValidateAsync(
        IReadOnlyDictionary<string, ZipArchiveEntry> entries,
        string entryPath,
        string expectedHash,
        CancellationToken cancellationToken)
    {
        var content = await ReadRequiredEntryAsync(
            entries,
            entryPath,
            cancellationToken);
        if (!string.Equals(
                Sha256(content),
                expectedHash,
                StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidDataException(
                $"Recovery archive checksum failed for '{entryPath}'.");
        }

        return content;
    }

    private static async Task<byte[]> ReadRequiredEntryAsync(
        IReadOnlyDictionary<string, ZipArchiveEntry> entries,
        string entryPath,
        CancellationToken cancellationToken)
    {
        if (!entries.TryGetValue(entryPath, out var entry))
        {
            throw new InvalidDataException(
                $"Recovery archive entry '{entryPath}' is missing.");
        }

        await using var stream = entry.Open();
        using var content = new MemoryStream();
        await stream.CopyToAsync(content, cancellationToken);
        return content.ToArray();
    }

    private static async Task WriteEntryAsync(
        ZipArchive archive,
        string entryPath,
        byte[] content,
        CancellationToken cancellationToken)
    {
        var entry = archive.CreateEntry(
            NormalizeArchivePath(entryPath),
            CompressionLevel.NoCompression);
        await using var stream = entry.Open();
        await stream.WriteAsync(content, cancellationToken);
    }

    private static string GetSafeDestination(
        string root,
        string relativePath)
    {
        ValidateRelativePath(relativePath, "Recovery asset");
        var fullRoot = Path.GetFullPath(root);
        var destination = Path.GetFullPath(
            Path.Combine(
                fullRoot,
                relativePath.Replace(
                    '/',
                    Path.DirectorySeparatorChar)));
        var rootPrefix = fullRoot.EndsWith(Path.DirectorySeparatorChar)
            ? fullRoot
            : $"{fullRoot}{Path.DirectorySeparatorChar}";

        if (!destination.StartsWith(
                rootPrefix,
                OperatingSystem.IsWindows()
                    ? StringComparison.OrdinalIgnoreCase
                    : StringComparison.Ordinal))
        {
            throw new InvalidDataException(
                $"Recovery asset '{relativePath}' escapes the data directory.");
        }

        return destination;
    }

    private static void ValidateRelativePath(
        string path,
        string label)
    {
        var normalized = NormalizeArchivePath(path);
        if (string.IsNullOrWhiteSpace(normalized)
            || Path.IsPathRooted(path)
            || normalized.Split('/').Any(part =>
                part is "." or ".." || string.IsNullOrWhiteSpace(part)))
        {
            throw new InvalidDataException(
                $"{label} path '{path}' is invalid.");
        }
    }

    private static string NormalizeArchivePath(string path)
        => path.Replace('\\', '/');

    private static string Sha256(byte[] content)
        => Convert.ToHexString(SHA256.HashData(content)).ToLowerInvariant();

    private static async Task<(Guid InstanceId, string ServerVersion)>
        ReadDatabaseMetadataAsync(
            string connectionString,
            CancellationToken cancellationToken)
    {
        await using var connection = new NpgsqlConnection(connectionString);
        await connection.OpenAsync(cancellationToken);
        await using var command = new NpgsqlCommand(
            """
            SELECT
                (SELECT instanceid FROM citadelinstanceidentity WHERE id = 1),
                current_setting('server_version');
            """,
            connection);
        await using var reader = await command.ExecuteReaderAsync(
            cancellationToken);
        if (!await reader.ReadAsync(cancellationToken))
        {
            throw new InvalidDataException(
                "Citadel instance identity is missing from the database.");
        }

        return (reader.GetGuid(0), reader.GetString(1));
    }

    private sealed record RecoveryAsset(
        string RelativePath,
        string SourcePath,
        bool Required);
}

internal sealed record ControlPlaneRecoveryManifest(
    int FormatVersion,
    string Product,
    string CoreInformationalVersion,
    string CompatibilityVersion,
    Guid InstanceId,
    DateTimeOffset CreatedAt,
    DatabaseManifest Database,
    IReadOnlyList<RecoveryAssetManifestEntry> Assets);

internal sealed record DatabaseManifest(
    string Engine,
    string DumpFile,
    string ServerVersion,
    string Sha256);

internal sealed record RecoveryAssetManifestEntry(
    string Name,
    string ArchivePath,
    bool Required,
    string Sha256);

[JsonSerializable(typeof(ControlPlaneRecoveryManifest))]
internal partial class RecoveryJsonContext : JsonSerializerContext;
