using Infrastructure.Entities;
using Infrastructure.Entities.Identity;
using Infrastructure.EntityFramework.Configurations;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Storage.ValueConversion;

namespace Infrastructure.EntityFramework;

public sealed class ApplicationDbContext(DbContextOptions<ApplicationDbContext> dbContextOptions)
    : DbContext(dbContextOptions)
{
    public DbSet<Platform> Platforms { get; private set; }
    public DbSet<PlatformStat> PlatformStats { get; private set; }
    public DbSet<ContainerInfo> ContainersInfo { get; private set; }
    public DbSet<ContainerStat> ContainerStats { get; private set; }
    public DbSet<User> Users { get; private set; }
    public DbSet<Team> Teams { get; private set; }
    public DbSet<Role> Roles { get; private set; }
    public DbSet<Permission> Permissions { get; private set; }
    public DbSet<UserTeam> UsersTeams { get; private set; }
    public DbSet<Registry> Registries { get; private set; }
    public DbSet<RefreshToken> RefreshTokens { get; private set; }

    /// <inheritdoc/>
    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        var guidToBlob = new ValueConverter<Guid, byte[]>(v => v.ToByteArray(), v => new Guid(v));
        foreach (var entity in modelBuilder.Model.GetEntityTypes())
        {
            foreach (var property in entity.GetProperties().Where(p => p.ClrType == typeof(Guid)))
            {
                property.SetValueConverter(guidToBlob);
                property.SetColumnType("BLOB"); // Store Guid as BLOB instead of TEXT
            }
        }

        modelBuilder.Ignore<ContainerPort>();
        modelBuilder.Ignore<RegistryConfigurationBase>();
        // modelBuilder.ApplyConfigurationsFromAssembly(GetType().Assembly); // breaks in trimming mode
        modelBuilder.ApplyConfiguration(new ContainerInfoConfiguration());
        modelBuilder.ApplyConfiguration(new ContainerStatConfiguration());
        modelBuilder.ApplyConfiguration(new PermissionConfiguration());
        modelBuilder.ApplyConfiguration(new PlatformConfiguration());
        modelBuilder.ApplyConfiguration(new PlatformStatConfiguration());
        modelBuilder.ApplyConfiguration(new RegistryConfiguration());
        modelBuilder.ApplyConfiguration(new RoleConfiguration());
        modelBuilder.ApplyConfiguration(new TeamConfiguration());
        modelBuilder.ApplyConfiguration(new UserConfiguration());
        modelBuilder.ApplyConfiguration(new UserTeamConfiguration());

    }

}