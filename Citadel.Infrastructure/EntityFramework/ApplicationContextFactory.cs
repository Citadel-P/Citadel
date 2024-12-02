using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;

namespace Infrastructure.EntityFramework;

/// <summary>
/// Used to init db and generate migrations, for more info <see cref="https://learn.microsoft.com/en-us/ef/core/cli/dbcontext-creation?tabs=dotnet-core-cli"/>
/// </summary>
internal sealed class ApplicationContextFactory : IDesignTimeDbContextFactory<ApplicationDbContext>
{
    internal const string DbFilePath = "./data/Citadel.db"; // path on the container
    internal const string ConnectionString = $"Data Source={DbFilePath}";

    public ApplicationDbContext CreateDbContext(string[] args)
    {
        var appDirectory = Directory.GetParent(Directory.GetCurrentDirectory()) + "/Citadel.WebApi/";
        if (!Directory.Exists(appDirectory))
        {
            throw new IOException($"Can't find the provided directory {appDirectory}");
        }
        
        var optionsBuilder = new DbContextOptionsBuilder<ApplicationDbContext>();
        optionsBuilder.UseSqlite(ConnectionString, config =>
        {
            config.CommandTimeout(60);
            config.UseQuerySplittingBehavior(QuerySplittingBehavior.SplitQuery);
        });

        return new(optionsBuilder.Options);
    }
}