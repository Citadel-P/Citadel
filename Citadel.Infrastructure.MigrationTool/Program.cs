using System.Text;
using FluentMigrator.Runner;
using Infrastructure.MigrationTool.Migrations;
using Microsoft.Extensions.DependencyInjection;
using FluentMigrator.Runner.Processors;
using FluentMigrator.Runner.Logging;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using System.Text.RegularExpressions;

public class Program
{
    public static async Task Main(string[] args)
    {
        // Determine which type of migration to generate based on command-line arguments
        bool generateSchema = args.Contains("--schema");
        bool generateSeed = args.Contains("--seed");
        bool generateFull = args.Contains("--full");

        if (!generateSchema && !generateSeed && !generateFull)
        {
            Console.WriteLine("Please specify what to generate: --schema, --seed or --full");
            return;
        }

        if (generateSchema)
        {
            await GenerateSchemaMigration();
        }

        if (generateSeed)
        {
            await GenerateSeedDataMigration();
        }

        if (generateFull)
        {
            await GenerateSchemaMigration();
            await GenerateSeedDataMigration();
        }
    }

    /// <summary>
    /// Generates an incremental SQL script for the database schema.
    /// This script will contain CREATE TABLE IF NOT EXISTS and CREATE INDEX IF NOT EXISTS statements.
    /// </summary>
    static async Task GenerateSchemaMigration()
    {
        // Output path for the new incremental SQL migration file
        string incrementalSqlFile = $"./../Citadel.Infrastructure/Scripts/{DateTime.UtcNow:yyyyMMddHHmmss}_Schema.sql";
        Directory.CreateDirectory(Path.GetDirectoryName(incrementalSqlFile)!);

        var migrationsAssembly = typeof(Migration0001).Assembly; // Your migrations assembly

        Console.WriteLine($"🔍 Generating schema SQL to: {incrementalSqlFile}");

        using (var sw = new StreamWriter(incrementalSqlFile, append: false))
        {
            var services = new ServiceCollection()
                .AddFluentMigratorCore()
                .ConfigureRunner(rb => rb
                    .AddSQLite()
                    .WithGlobalConnectionString("Data Source=:memory:")
                    .ScanIn(migrationsAssembly).For.Migrations())
                .Configure<ProcessorOptions>(opt =>
                {
                    opt.PreviewOnly = true;
                    opt.Timeout = TimeSpan.FromSeconds(60);
                })
                .Configure<SqlScriptFluentMigratorLoggerOptions>(opt =>
                {
                    opt.OutputGoBetweenStatements = false;
                    opt.ShowSql = true;
                })
                .AddSingleton<ILoggerProvider>(sp => new SqlScriptFluentMigratorLoggerProvider(
                    sw,
                    sp.GetRequiredService<IOptions<SqlScriptFluentMigratorLoggerOptions>>().Value
                ))
                .AddLogging(lb => lb.AddFluentMigratorConsole())
                .BuildServiceProvider(false);

            using var scope = services.CreateScope();
            var runner = scope.ServiceProvider.GetRequiredService<IMigrationRunner>();

            // This will generate SQL for all migrations it considers "pending"
            runner.MigrateUp();
        }

        await Task.Delay(100);

        Console.WriteLine($"Applying cleaning filters to: {incrementalSqlFile}");
        string rawGeneratedSql = await File.ReadAllTextAsync(incrementalSqlFile);

        // Clean the schema SQL: remove VersionInfo, comments, ensure idempotency for CREATE TABLE/INDEX, and remove seed data INSERTS
        string cleanSql = CleanGeneratedSchemaSql(rawGeneratedSql, Path.GetFileName(incrementalSqlFile));

        await File.WriteAllTextAsync(incrementalSqlFile, cleanSql);

        Console.WriteLine($"✅ Generated schema SQL to: {incrementalSqlFile}");
    }

    /// <summary>
    /// Generates an SQL script containing only idempotent INSERT statements for seed data.
    /// </summary>
    static async Task GenerateSeedDataMigration()
    {
        // Output path for the new seed data SQL migration file
        string seedDataSqlFile = $"./../Citadel.Infrastructure/Scripts/{DateTime.UtcNow:yyyyMMddHHmmss}_SeedData.sql";
        Directory.CreateDirectory(Path.GetDirectoryName(seedDataSqlFile)!);

        // We only want to run the SeedInitialData migration here
        var seedMigrationsAssembly = typeof(SeedInitialData).Assembly;

        Console.WriteLine($"🔍 Generating seed data SQL to: {seedDataSqlFile}");

        using (var sw = new StreamWriter(seedDataSqlFile, append: false))
        {
            var services = new ServiceCollection()
                .AddFluentMigratorCore()
                .ConfigureRunner(rb => rb
                    .AddSQLite()
                    .WithGlobalConnectionString("Data Source=:memory:")
                    .ScanIn(seedMigrationsAssembly).For.Migrations())
                .Configure<ProcessorOptions>(opt =>
                {
                    opt.PreviewOnly = true;
                    opt.Timeout = TimeSpan.FromSeconds(60);
                })
                .Configure<SqlScriptFluentMigratorLoggerOptions>(opt =>
                {
                    opt.OutputGoBetweenStatements = false;
                    opt.ShowSql = true;
                })
                .AddSingleton<ILoggerProvider>(sp => new SqlScriptFluentMigratorLoggerProvider(
                    sw,
                    sp.GetRequiredService<IOptions<SqlScriptFluentMigratorLoggerOptions>>().Value
                ))
                .AddLogging(lb => lb.AddFluentMigratorConsole())
                .BuildServiceProvider(false);

            using var scope = services.CreateScope();
            var runner = scope.ServiceProvider.GetRequiredService<IMigrationRunner>();

            // We want to run only the SeedInitialData migration.
            // FluentMigrator's MigrateUp() will run all pending.
            // A more precise way would be to target a specific version:
            // runner.MigrateUp(typeof(SeedInitialData).GetCustomAttribute<MigrationAttribute>()!.Version);
            // However, this might require more complex setup for dependencies.
            // For now, we'll let it run all and filter aggressively.
            runner.MigrateUp();
        }

        await Task.Delay(100);

        Console.WriteLine($"Applying cleaning filters to: {seedDataSqlFile}");
        string rawGeneratedSql = await File.ReadAllTextAsync(seedDataSqlFile);

        // Clean the seed data SQL: remove all DDL, VersionInfo, comments, and make INSERTS idempotent
        string cleanSql = CleanGeneratedSeedDataSql(rawGeneratedSql, Path.GetFileName(seedDataSqlFile));

        await File.WriteAllTextAsync(seedDataSqlFile, cleanSql);

        Console.WriteLine($"✅ Generated seed data SQL to: {seedDataSqlFile}");
    }

    /// <summary>
    /// Cleans the raw SQL generated by FluentMigrator for schema migrations.
    /// Filters out VersionInfo, comments, and seed data INSERTS. Makes CREATE TABLE/INDEX idempotent.
    /// </summary>
    static string CleanGeneratedSchemaSql(string rawSql, string filename)
    {
        var lines = rawSql.Split(new[] { '\n', '\r' }, StringSplitOptions.RemoveEmptyEntries);
        var clean = new StringBuilder();

        clean.AppendLine($"-- Migration: {filename}");
        clean.AppendLine($"-- Generated: {DateTime.UtcNow:yyyy-MM-dd HH:mm:ss} UTC");
        clean.AppendLine();

        // Tables that contain seed data and whose INSERT statements should be SKIPPED in schema file
        var seedDataTables = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "Roles", "Users", "Teams", "Permissions", "UsersTeams"
        };

        var createIndexRegex = new Regex(@"^CREATE (UNIQUE )?INDEX\s", RegexOptions.IgnoreCase | RegexOptions.Compiled);

        foreach (var line in lines)
        {
            var trimmed = line.Trim();

            // Skip FluentMigrator's internal VersionInfo table related statements
            if (trimmed.Contains("VersionInfo", StringComparison.OrdinalIgnoreCase)) continue;
            // Skip FluentMigrator's specific action comments (e.g., /* CreateTable ... */)
            if (trimmed.StartsWith("/*") && trimmed.EndsWith("*/")) continue;
            // Skip SQLite-specific boilerplate or common batch separators
            if (trimmed.StartsWith("PRAGMA", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("BEGIN TRANSACTION", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("COMMIT", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("DROP TABLE IF EXISTS sqlite_sequence", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("INSERT INTO sqlite_sequence", StringComparison.OrdinalIgnoreCase) ||
                trimmed.Equals("GO", StringComparison.OrdinalIgnoreCase) ||
                string.IsNullOrWhiteSpace(trimmed)) continue;

            string processedLine = trimmed;

            // Make CREATE TABLE statements idempotent
            if (processedLine.StartsWith("CREATE TABLE", StringComparison.OrdinalIgnoreCase) && !processedLine.Contains("IF NOT EXISTS", StringComparison.OrdinalIgnoreCase))
            {
                processedLine = processedLine.Replace("CREATE TABLE", "CREATE TABLE IF NOT EXISTS");
            }
            // Make CREATE INDEX statements idempotent
            else if (createIndexRegex.IsMatch(processedLine) && !processedLine.Contains("IF NOT EXISTS", StringComparison.OrdinalIgnoreCase))
            {
                processedLine = createIndexRegex.Replace(processedLine, match => $"CREATE {match.Groups[1].Value}INDEX IF NOT EXISTS ", 1);
            }
            // Skip INSERT statements for seed data tables (they belong in the seed data script)
            else if (processedLine.StartsWith("INSERT INTO", StringComparison.OrdinalIgnoreCase))
            {
                var tableNameMatch = Regex.Match(processedLine, @"INSERT INTO ""?(\w+)""?");
                if (tableNameMatch.Success && seedDataTables.Contains(tableNameMatch.Groups[1].Value))
                {
                    continue; 
                }
            }

            clean.AppendLine(processedLine);
            // Add a semicolon if the line is not empty and doesn't already end with one
            if (!string.IsNullOrWhiteSpace(processedLine) && !processedLine.EndsWith(";"))
            {
                clean.AppendLine(";");
            }
        }
        return clean.ToString();
    }

    /// <summary>
    /// Cleans the raw SQL generated by FluentMigrator for seed data migrations.
    /// Filters out all DDL, VersionInfo, comments, and makes relevant INSERTS idempotent.
    /// </summary>
    static string CleanGeneratedSeedDataSql(string rawSql, string filename)
    {
        var lines = rawSql.Split(new[] { '\n', '\r' }, StringSplitOptions.RemoveEmptyEntries);
        var clean = new StringBuilder();

        clean.AppendLine($"-- Migration: {filename}");
        clean.AppendLine($"-- Generated: {DateTime.UtcNow:yyyy-MM-dd HH:mm:ss} UTC");
        clean.AppendLine();

        // Tables that contain seed data and whose INSERT statements should be made idempotent
        var seedDataTables = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
        {
            "Roles", "Users", "Teams", "Permissions", "UsersTeams"
        };

        foreach (var line in lines)
        {
            var trimmed = line.Trim();

            // Skip any DDL statements (CREATE TABLE, CREATE INDEX, ALTER TABLE, DROP TABLE, etc.)
            if (trimmed.StartsWith("CREATE ", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("ALTER ", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("DROP ", StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            // Skip FluentMigrator's internal VersionInfo table related statements
            if (trimmed.Contains("VersionInfo", StringComparison.OrdinalIgnoreCase)) continue;
            // Skip FluentMigrator's specific action comments (e.g., /* CreateTable ... */)
            if (trimmed.StartsWith("/*") && trimmed.EndsWith("*/")) continue;
            // Skip SQLite-specific boilerplate or common batch separators
            if (trimmed.StartsWith("PRAGMA", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("BEGIN TRANSACTION", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("COMMIT", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("DROP TABLE IF EXISTS sqlite_sequence", StringComparison.OrdinalIgnoreCase) ||
                trimmed.StartsWith("INSERT INTO sqlite_sequence", StringComparison.OrdinalIgnoreCase) ||
                trimmed.Equals("GO", StringComparison.OrdinalIgnoreCase) ||
                string.IsNullOrWhiteSpace(trimmed)) continue;

            string processedLine = trimmed;

            // Process only INSERT statements for known seed data tables
            if (processedLine.StartsWith("INSERT INTO", StringComparison.OrdinalIgnoreCase))
            {
                var tableNameMatch = Regex.Match(processedLine, @"INSERT INTO ""?(\w+)""?");
                if (tableNameMatch.Success && seedDataTables.Contains(tableNameMatch.Groups[1].Value))
                {
                    // Make INSERT statements idempotent (INSERT OR IGNORE)
                    processedLine = processedLine.Replace("INSERT INTO", "INSERT OR IGNORE INTO");
                }
                else
                {
                    // If it's an INSERT but not for a known seed data table, skip it
                    continue;
                }
            }
            else
            {
                // Skip any other non-INSERT statements
                continue;
            }

            clean.AppendLine(processedLine);
            // Add a semicolon if the line is not empty and doesn't already end with one
            if (!string.IsNullOrWhiteSpace(processedLine) && !processedLine.EndsWith(";"))
            {
                clean.AppendLine(";");
            }
        }
        return clean.ToString();
    }
}
