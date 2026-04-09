using Hosting.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;

namespace Infrastructure.Migrations.EntityFramework;

internal sealed class ApplicationContextFactory : IDesignTimeDbContextFactory<ApplicationDbContext>
{
    public ApplicationDbContext CreateDbContext(string[] args)
    {
        var optionsBuilder = new DbContextOptionsBuilder<ApplicationDbContext>();

        optionsBuilder.UseNpgsql(Constants.ConnectionString, o => {
            o.EnableRetryOnFailure();
        });

        return new(optionsBuilder.Options);
    }
}
