using System.Data.Common;
using Hosting.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;
using Microsoft.EntityFrameworkCore.Diagnostics;

namespace Infrastructure.EntityFramework;

/// <summary>
/// Warn: Used only to generate migrations, for more info <see cref="https://learn.microsoft.com/en-us/ef/core/cli/dbcontext-creation?tabs=dotnet-core-cli"/>
/// </summary>
internal sealed class ApplicationContextFactory : IDesignTimeDbContextFactory<ApplicationDbContext>
{
    internal const string ConnectionString = $"Data Source={Constants.DbFilePath};Cache=Shared;";

    public ApplicationDbContext CreateDbContext(string[] args)
    {
        var appDirectory = Directory.GetParent(Directory.GetCurrentDirectory()) + "/Citadel.WebApi/";
        if (!Directory.Exists(appDirectory))
        {
            throw new IOException($"Can't find the provided directory {appDirectory}");
        }

        var optionsBuilder = new DbContextOptionsBuilder<ApplicationDbContext>();
        optionsBuilder.UseSqlite(ConnectionString);

        return new(optionsBuilder.Options);
    }
}

internal class SqlitePragmaInterceptor : DbConnectionInterceptor
{
    public override void ConnectionOpened(DbConnection connection, ConnectionEndEventData eventData)
    {
        using var command = connection.CreateCommand();
        // Set PRAGMA settings for SQLite to improve concurrency
        command.CommandText = @"
            PRAGMA journal_mode=WAL;
            PRAGMA synchronous=NORMAL;
            PRAGMA busy_timeout=3000;
        ";
        command.ExecuteNonQuery();
    }
}