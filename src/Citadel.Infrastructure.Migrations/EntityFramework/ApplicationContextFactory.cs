using Hosting.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;

namespace Infrastructure.Migrations.EntityFramework;

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
