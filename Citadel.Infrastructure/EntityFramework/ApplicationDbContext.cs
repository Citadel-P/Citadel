using Infrastructure.Entities;
using Infrastructure.Entities.Identity;
using Infrastructure.EntityFramework.JoiningTables;
using Microsoft.EntityFrameworkCore;

namespace Infrastructure.EntityFramework;

public sealed class ApplicationDbContext(DbContextOptions<ApplicationDbContext> dbContextOptions)
    : DbContext(dbContextOptions)
{
    public DbSet<Platform> Platforms { get; private set; }
    public DbSet<PlatformStat> PlatformStats { get; private set; }
    public DbSet<ContainerInfo> ContainersInfo { get; private set; }
    public DbSet<ContainerStat> ContainerStats { get; private set; }
    public DbSet<Entities.SystemInfo> SystemsInfo { get; private set; }
    public DbSet<Entities.SwarmInfo> SwarmsInfo { get; private set; }
    public DbSet<SwarmPeer> SwarmsPeer { get; private set; }
    public DbSet<User> Users { get; private set; }
    public DbSet<Team> Teams { get; private set; }
    public DbSet<Role> Roles { get; private set; }
    public DbSet<Permission> Permissions { get; private set; }
    public DbSet<UserTeam> UsersTeams { get; private set; }
    public DbSet<Registry> Registries { get; private set; }

    /// <inheritdoc/>
    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.ApplyConfigurationsFromAssembly(GetType().Assembly);
    }

    /// <inheritdoc/>
    public override async Task<int> SaveChangesAsync(CancellationToken cancellationToken = default)
    {
        int result = await base.SaveChangesAsync(cancellationToken);
        foreach (var entry in ChangeTracker.Entries())
        {
            entry.State = EntityState.Detached;
        }

        return result;
    }
}