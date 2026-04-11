using Domain;
using Hosting.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using System.Security.Cryptography;
using System.Text;

namespace Infrastructure.Migrations.EntityFramework;

internal sealed class ApplicationDbContext(DbContextOptions<ApplicationDbContext> dbContextOptions)
    : DbContext(dbContextOptions)
{
    /// <inheritdoc/>
    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder
            .ContainerConfiguration()
            .ContainerStatConfiguration()
            .PlatformConfiguration()
            .PlatformStatConfiguration()
            .RegistryConfiguration()
            .GitAccountConfiguration()
            .GitRepositoryConfiguration()
            .RefreshTokenConfiguration()
            .ActorConfiguration()
            .UserConfiguration()
            .TeamConfiguration()
            .UserTeamConfiguration()
            .PermissionConfiguration()
            .ResourceAccessConfiguration()
            .RoleConfiguration()
            .ActorRoleConfiguration()
            .DeploymentConfiguration()
            .ImageConfiguration()
            .ActivityEventConfiguration()
            .AlertRuleConfiguration()
            .AlertEventConfiguration()
            .AlertRuleStateConfiguration()
            .AlertChannelConfiguration()
            .StackConfiguration()
            .StackReleaseConfiguration()
            .AlertRuleChannelConfiguration();

        SeedDb(modelBuilder);
        ToLower(modelBuilder);
    }

    private static void SeedDb(ModelBuilder modelBuilder)
    {
        var seedDate = new DateTime(2026, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        // Actors
        var systemActorId = Constants.SystemId;
        var adminActorId = Constants.DefaultAdminId;
        var teamActorId = Constants.TeamActorId;

        // Users / Teams
        var adminUserId = Guid.Parse("10000000-0000-0000-0000-000000000001");
        var teamId = Guid.Parse("20000000-0000-0000-0000-000000000001");

        // Roles
        var adminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");
        var operatorRoleId = Guid.Parse("30000000-0000-0000-0000-000000000002");
        var viewerRoleId = Guid.Parse("30000000-0000-0000-0000-000000000003");

        // Actors
        modelBuilder.Entity("Actor").HasData(
            new { Id = systemActorId, Type = "System", IsEnabled = true },
            new { Id = adminActorId, Type = "User", IsEnabled = true },
            new { Id = teamActorId, Type = "Team", IsEnabled = true }
        );

        // User
        modelBuilder.Entity("User").HasData(new
        {
            Id = adminUserId,
            Name = "admin",
            Email = "admin@citadel.local",
            Password = "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1",
            ActorId = adminActorId,
            CreatedAt = seedDate,
            CreatedByActorId = systemActorId
        });

        // Team
        modelBuilder.Entity("Team").HasData(new
        {
            Id = teamId,
            Name = "Default Team",
            ActorId = teamActorId
        });

        // Roles
        modelBuilder.Entity("Role").HasData(
            new { Id = adminRoleId, Name = "Admin" },
            new { Id = operatorRoleId, Name = "Operator" },
            new { Id = viewerRoleId, Name = "Viewer" }
        );

        // ActorRoles
        modelBuilder.Entity("ActorRole").HasData(
            // Admin user -> Admin role
            new { ActorId = adminActorId, RoleId = adminRoleId },

            // Team -> Operator role
            new { ActorId = teamActorId, RoleId = operatorRoleId }
        );

        // Permissions
        var permissions = new List<object>();

        foreach (var resource in Enum.GetValues<ResourceType>())
        {
            foreach (var action in Enum.GetValues<ResourceAction>())
            {
                // Admin -> everything
                permissions.Add(new
                {
                    Id = CreatePermissionSeedId(adminRoleId, resource, action),
                    RoleId = adminRoleId,
                    ResourceType = resource.ToString(),
                    ResourceAction = action.ToString()
                });

                // Viewer -> only View
                if (action == ResourceAction.View)
                {
                    permissions.Add(new
                    {
                        Id = CreatePermissionSeedId(viewerRoleId, resource, action),
                        RoleId = viewerRoleId,
                        ResourceType = resource.ToString(),
                        ResourceAction = action.ToString()
                    });
                }

                // Operator -> most except Delete (example policy)
                if (action != ResourceAction.Delete)
                {
                    permissions.Add(new
                    {
                        Id = CreatePermissionSeedId(operatorRoleId, resource, action),
                        RoleId = operatorRoleId,
                        ResourceType = resource.ToString(),
                        ResourceAction = action.ToString()
                    });
                }
            }
        }

        modelBuilder.Entity("Permission").HasData(permissions);

        // --- Alert Rules ---
        modelBuilder.Entity("AlertRule").HasData(
            // Platform threshold alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000001"), Name = "CPU > 90% - Platform", Type = "PlatformCpuHigh", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)90.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000011"), Name = "CPU > 80% - Platform", Type = "PlatformCpuHigh", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)80.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000002"), Name = "RAM > 90% - Platform", Type = "PlatformRamHigh", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)90.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000022"), Name = "RAM > 80% - Platform", Type = "PlatformRamHigh", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)80.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Platform event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000003"), Name = "Platform Unreachable", Type = "PlatformUnreachable", Severity = "Critical", CooldownSeconds = 600, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000004"), Name = "Platform Version Mismatch", Type = "PlatformVersionMismatch", Severity = "Warning", CooldownSeconds = 3600, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000005"), Name = "Unmanaged Container Created", Type = "UnmanagedContainerCreated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Deployment event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000006"), Name = "Image Update Available - Deployment", Type = "DeploymentImageUpdateAvailable", Severity = "Info", CooldownSeconds = 60 * 60 * 24, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000007"), Name = "Auto Deploy Failed - Deployment", Type = "DeploymentAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000008"), Name = "Deployment Auto Updated", Type = "DeploymentAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Stack event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000009"), Name = "Image Update Available - Stack", Type = "StackImageUpdateAvailable", Severity = "Info", CooldownSeconds = 60 * 60 * 24, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000a"), Name = "Auto Deploy Failed - Stack", Type = "StackAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000b"), Name = "Stack Auto Updated", Type = "StackAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate }
        );

        // --- Registries ---
        modelBuilder.Entity("Registry").HasData(new
        {
            Id = Constants.DefaultRegistryId,
            Name = Constants.DefaultRegistryName,
            Description = "Public Docker Hub Registry",
            RegistryHost = "hub.docker.com",
            CreatedByActorId = Constants.SystemId,
            CreatedAt = seedDate,
            Status = "Active",
            Configuration = """
            {
                "$type": "DockerHub"
            }
            """
        });
    }

    private static Guid CreatePermissionSeedId(Guid roleId, ResourceType resourceType, ResourceAction resourceAction)
    {
        var input = $"permission:{roleId:D}:{resourceType}:{resourceAction}";
        var hash = SHA256.HashData(Encoding.UTF8.GetBytes(input));
        var hex = Convert.ToHexString(hash);

        return Guid.ParseExact(
            $"{hex[..8]}-{hex[8..12]}-{hex[12..16]}-{hex[16..20]}-{hex[20..32]}",
            "D");
    }

    private static void ToLower(ModelBuilder modelBuilder)
    {
        foreach (var entity in modelBuilder.Model.GetEntityTypes())
        {
            entity.SetTableName(entity.GetTableName()?.ToLowerInvariant());

            foreach (var property in entity.GetProperties())
            {
                property.SetColumnName(property.GetColumnName()?.ToLowerInvariant());
            }

            foreach (var key in entity.GetKeys())
                key.SetName(key.GetName()?.ToLowerInvariant());

            foreach (var foreignKey in entity.GetForeignKeys())
                foreignKey.SetConstraintName(foreignKey.GetConstraintName()?.ToLowerInvariant());

            foreach (var index in entity.GetIndexes())
                index.SetDatabaseName(index.GetDatabaseName()?.ToLowerInvariant());
        }
    }
}

internal static class Configuration
{
    private const string Text = "text";
    private const string Json = "json";
    private const string Integer = "integer";
    private const string BigInt = "bigint";
    private const string Double = "double precision";
    private const string Timestamp = "timestamp with time zone";

    public static ModelBuilder ContainerConfiguration(this ModelBuilder builder)
    {
        var tableName = "Containers";
        var container = builder.Entity("Container");

        container.ToTable(tableName);

        container.Property<Guid>("Id").IsRequired();
        container.HasKey("Id");

        container.Property<Guid>("PlatformId").IsRequired();
        container.Property<Guid?>("DeploymentId").IsRequired(false);
        container.Property<Guid?>("ImageId").IsRequired(false);
        container.Property<string>("DockerContainerId").HasColumnType(Text).IsRequired().HasMaxLength(64);
        container.Property<string>("DockerImageId").HasColumnType(Text).IsRequired();
        container.Property<string>("Name").HasColumnType(Text).IsRequired();
        container.Property<long>("Created").HasColumnType(BigInt).IsRequired();
        container.Property<long>("Updated").HasColumnType(BigInt).IsRequired();
        container.Property<string>("State").HasColumnType(Text).IsRequired();
        container.Property<string>("Stack").HasColumnType(Text);
        container.Property<string>("Ports").HasColumnType(Json).IsRequired();

        container.AddReconcilableMember();

        container
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        container
            .HasOne("Image")
            .WithMany()
            .IsRequired(false)
            .HasForeignKey("ImageId")
            .OnDelete(DeleteBehavior.SetNull);

        container
            .HasOne("Deployment")
            .WithMany()
            .IsRequired(false)
            .HasForeignKey("DeploymentId")
            .OnDelete(DeleteBehavior.SetNull);

        container.HasIndex("DockerContainerId", "PlatformId").IsUnique().HasDatabaseName($"IX__{tableName}_DockerContainerId_PlatformId");
        container.HasIndex("ImageId").HasDatabaseName($"IX_{tableName}_ImageId");
        container.HasIndex("PlatformId").HasDatabaseName($"IX_{tableName}_PlatformId");
        container.HasIndex("DockerImageId").HasDatabaseName($"IX_{tableName}_DockerImageId");

        return builder;
    }

    public static ModelBuilder GitRepositoryConfiguration(this ModelBuilder builder)
    {
        var tableName = "GitRepositories";
        var gitRepository = builder.Entity("GitRepository");

        gitRepository.ToTable(tableName);

        gitRepository.Property<Guid>("Id").IsRequired();
        gitRepository.HasKey("Id");

        gitRepository.Property<string>("Name").HasColumnType(Text).IsRequired();
        gitRepository.Property<string>("Description").HasColumnType(Text).IsRequired(false).HasMaxLength(600);
        gitRepository.Property<string>("Url").HasColumnType(Text).IsRequired();
        gitRepository.Property<string>("DefaultBranch").HasColumnType(Text).IsRequired();
        gitRepository.Property<string>("Status").HasColumnType(Text).IsRequired();
        gitRepository.Property<Guid?>("GitAccountId").IsRequired(false);
        gitRepository.Property<bool>("WebHookEnabled").HasColumnType("boolean").IsRequired().HasDefaultValue(false);
        gitRepository.Property<string>("WebHookSecret").HasColumnType(Text).IsRequired(false);
        gitRepository.Property<string>("OnClone").HasColumnType(Text).IsRequired(false);
        gitRepository.Property<string>("OnPull").HasColumnType(Text).IsRequired(false);

        gitRepository
            .AddReconcilableMember()
            .AddAuditedMemebers();

        gitRepository
            .HasOne("GitAccount")
            .WithMany()
            .IsRequired(false)
            .HasForeignKey("GitAccountId")
            .OnDelete(DeleteBehavior.SetNull);

        gitRepository.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");
        gitRepository.HasIndex("GitAccountId").HasDatabaseName($"IX_{tableName}_GitAccountId");

        return builder;
    }

    public static ModelBuilder GitAccountConfiguration(this ModelBuilder builder)
    {
        var tableName = "GitAccounts";
        var gitAccount = builder.Entity("GitAccount");

        gitAccount.ToTable(tableName);

        gitAccount.Property<Guid>("Id").IsRequired();
        gitAccount.HasKey("Id");

        gitAccount.Property<string>("Name").HasColumnType(Text).IsRequired();
        gitAccount.Property<string>("Domain").HasColumnType(Text).IsRequired();
        gitAccount.Property<string>("Transport").HasColumnType(Text).IsRequired();
        gitAccount.Property<string>("AuthType").HasColumnType(Text).IsRequired();
        gitAccount.Property<string>("Configuration").HasColumnType(Json).IsRequired();

        gitAccount.AddAuditedMemebers();

        gitAccount.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");

        return builder;
    }

    public static ModelBuilder PlatformConfiguration(this ModelBuilder builder)
    {
        var tableName = "Platforms";
        var platform = builder.Entity("Platform");

        platform.ToTable(tableName);

        platform.Property<Guid>("Id").IsRequired();
        platform.HasKey("Id");

        platform.Property<string>("Name").HasColumnType(Text).IsRequired();
        platform.Property<string>("Address").HasColumnType(Text).IsRequired();
        platform.Property<string>("Status").HasColumnType(Text).IsRequired();
        platform.Property<string>("ConnectorType").HasColumnType(Text).IsRequired();
        platform.Property<int>("NetworkCount").HasColumnType(Integer).IsRequired();
        platform.Property<int>("VolumeCount").HasColumnType(Integer).IsRequired();
        platform.Property<int>("ImageCount").HasColumnType(Integer).IsRequired();
        platform.Property<int>("CpuCount").HasColumnType(Integer).IsRequired();
        platform.Property<long>("MemTotal").HasColumnType(BigInt).IsRequired();
        platform.Property<string>("AgentVersion").HasColumnType(Text);
        platform.Property<string>("ServerVersion").HasColumnType(Text);
        platform.Property<string>("PlatformDescriptor").HasColumnType(Json).IsRequired();

        platform.HasIndex("Address").IsUnique().HasDatabaseName($"IX_{tableName}_Address");

        return builder;
    }

    public static ModelBuilder ContainerStatConfiguration(this ModelBuilder builder)
    {
        var tableName = "ContainerStats";
        var stat = builder.Entity("ContainerStat");

        stat.ToTable(tableName);

        stat.Property<Guid>("Id").IsRequired();
        stat.HasKey("Id");

        stat.Property<Guid>("ContainerId").IsRequired();
        stat.Property<long>("Created").HasColumnType(BigInt).IsRequired();
        stat.Property<double>("MemoryActive").HasColumnType(Double);
        stat.Property<double>("MemoryCache").HasColumnType(Double);
        stat.Property<double>("CpuUsage").HasColumnType(Double);
        stat.Property<double>("MemoryLimit").HasColumnType(Double);
        stat.Property<double>("RxBytes").HasColumnType(Double);
        stat.Property<double>("TxBytes").HasColumnType(Double);

        stat
            .HasOne("Container")
            .WithMany()
            .HasForeignKey("ContainerId")
            .OnDelete(DeleteBehavior.Cascade);

        stat.HasIndex("ContainerId", "Created").IsUnique()
            .HasDatabaseName($"IX_{tableName}_ContainerId_Created");

        return builder;
    }

    public static ModelBuilder PlatformStatConfiguration(this ModelBuilder builder) 
    {
        var tableName = "PlatformStats";
        var stat = builder.Entity("PlatformStat");

        stat.ToTable(tableName);

        stat.Property<Guid>("Id").IsRequired();
        stat.HasKey("Id");

        stat.Property<Guid>("PlatformId").IsRequired();
        stat.Property<long>("Created").HasColumnType(BigInt).IsRequired();
        stat.Property<double>("MemoryUsage").HasColumnType(Double).IsRequired();
        stat.Property<double>("CpuUsage").HasColumnType(Double).IsRequired();
        stat.Property<double>("RxBytes").HasColumnType(Double).IsRequired();
        stat.Property<double>("TxBytes").HasColumnType(Double).IsRequired();

        stat
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        stat.HasIndex("PlatformId", "Created").IsUnique()
            .HasDatabaseName($"IX_{tableName}_PlatformId_Created");

        return builder;
    }

    public static ModelBuilder RegistryConfiguration(this ModelBuilder builder)
    {
        var tableName = "Registries";
        var registry = builder.Entity("Registry");

        registry.ToTable(tableName);

        registry.Property<Guid>("Id").IsRequired();
        registry.HasKey("Id");

        registry.Property<string>("Name").HasColumnType(Text).IsRequired();
        registry.Property<string>("Description").HasColumnType(Text).IsRequired(false).HasMaxLength(600);
        registry.Property<string>("RegistryHost").HasColumnType(Text).IsRequired();
        registry.Property<string>("Status").HasColumnType(Text).IsRequired();
        registry.Property<string>("Configuration").HasColumnType(Json).IsRequired();

        registry.AddAuditedMemebers();

        registry.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");

        return builder;
    }

    public static ModelBuilder RefreshTokenConfiguration(this ModelBuilder builder)
    {
        var tableName = "RefreshTokens";
        var refreshToken = builder.Entity("RefreshToken");

        refreshToken.ToTable(tableName);

        refreshToken.Property<Guid>("Id").IsRequired();
        refreshToken.HasKey("Id");

        refreshToken.Property<Guid>("UserId").IsRequired();
        refreshToken.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        refreshToken
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        refreshToken.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");

        return builder;
    }

    public static ModelBuilder ActorConfiguration(this ModelBuilder builder)
    {
        var tableName = "Actors";
        var actor = builder.Entity("Actor");

        actor.ToTable(tableName);

        actor.Property<Guid>("Id").IsRequired();
        actor.HasKey("Id");

        actor.Property<string>("Type").HasColumnType(Text).IsRequired();
        actor.Property<bool>("IsEnabled").HasColumnType("boolean").HasDefaultValue(true).IsRequired();

        return builder;
    }

    public static ModelBuilder UserConfiguration(this ModelBuilder builder)
    {
        var tableName = "Users";
        var user = builder.Entity("User");

        user.ToTable(tableName);

        user.Property<Guid>("Id").IsRequired();
        user.HasKey("Id");

        user.Property<Guid>("ActorId").IsRequired();
        user.Property<string>("Name").HasColumnType(Text).HasMaxLength(100).IsRequired();
        user.Property<string>("Email").HasColumnType(Text).IsRequired(false);
        user.Property<string>("Password").HasColumnType(Text).IsRequired(false);
        user.AddAuditedMemebers();

        user.HasIndex("Email")
            .IsUnique()
            .HasDatabaseName($"IX_{tableName}_Email");

        user
            .HasOne("Actor")
            .WithOne()
            .HasForeignKey("User", "ActorId")
            .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }

    public static ModelBuilder TeamConfiguration(this ModelBuilder builder)
    {
        var tableName = "Teams";
        var team = builder.Entity("Team");

        team.ToTable(tableName);

        team.Property<Guid>("Id").IsRequired();
        team.Property<Guid>("ActorId").IsRequired();

        team.HasKey("Id");

        team.Property<string>("Name").HasColumnType(Text).IsRequired();

        team
           .HasOne("Actor")
           .WithOne()
           .HasForeignKey("Team", "ActorId")
           .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }

    public static ModelBuilder UserTeamConfiguration(this ModelBuilder builder)
    {
        var tableName = "UsersTeams";
        var userTeam = builder.Entity("UserTeam");

        userTeam.ToTable(tableName);

        userTeam.Property<Guid>("UserId").IsRequired();
        userTeam.Property<Guid>("TeamId").IsRequired();
        userTeam.HasKey("UserId", "TeamId");
        userTeam
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);
        userTeam
            .HasOne("Team")
            .WithMany()
            .HasForeignKey("TeamId")
            .OnDelete(DeleteBehavior.Cascade);
        userTeam.HasIndex("TeamId").HasDatabaseName($"IX_{tableName}_TeamId");
        userTeam.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");

        return builder;
    }

    public static ModelBuilder PermissionConfiguration(this ModelBuilder builder)
    {
        var tableName = "Permissions";
        var permission = builder.Entity("Permission");

        permission.ToTable(tableName);
        
        permission.Property<Guid>("Id").IsRequired();
        permission.HasKey("Id");
        
        permission.Property<Guid>("RoleId").IsRequired();
        permission.Property<string>("ResourceType").HasColumnType(Text).IsRequired();
        permission.Property<string>("ResourceAction").HasColumnType(Text).IsRequired();

        permission
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("RoleId")
            .OnDelete(DeleteBehavior.Cascade);

        permission.HasIndex("RoleId").HasDatabaseName($"IX_{tableName}_RoleId");

        return builder;
    }

    public static ModelBuilder ResourceAccessConfiguration(this ModelBuilder builder)
    {
        var tableName = "ResourceAccesses";
        var resourceAccess = builder.Entity("ResourceAccess");

        resourceAccess.ToTable(tableName);

        resourceAccess.Property<Guid>("Id").IsRequired();
        resourceAccess.HasKey("Id");

        resourceAccess.Property<Guid>("ResourceId").IsRequired();
        resourceAccess.Property<Guid>("ActorId").IsRequired();
        resourceAccess.Property<string>("ResourceType").HasColumnType(Text).IsRequired();
        resourceAccess.Property<string>("Action").HasColumnType(Text).IsRequired();

        resourceAccess.HasIndex(
            "ResourceType",
            "ResourceId",
            "ActorId",
            "Action")
        .IsUnique();

        resourceAccess.HasIndex(
            "ResourceType",
            "ResourceId",
            "ActorId");

        resourceAccess.HasIndex("ActorId").HasDatabaseName($"IX_{tableName}_Actor"); ;

        return builder;
    }

    public static ModelBuilder RoleConfiguration(this ModelBuilder builder)
    {
        var tableName = "Roles";
        var role = builder.Entity("Role");

        role.ToTable(tableName);

        role.Property<Guid>("Id").IsRequired();
        role.HasKey("Id");

        role.Property<string>("Name").HasColumnType(Text).IsRequired();

        return builder;
    }

    public static ModelBuilder ActorRoleConfiguration(this ModelBuilder builder)
    {
        var tableName = "ActorRoles";
        var actorRole = builder.Entity("ActorRole");

        actorRole.ToTable(tableName);

        actorRole.Property<Guid>("ActorId").IsRequired();
        actorRole.Property<Guid>("RoleId").IsRequired();
        actorRole.HasKey("ActorId", "RoleId");

        actorRole
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("ActorId")
            .OnDelete(DeleteBehavior.Cascade);
        actorRole
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("RoleId")
            .OnDelete(DeleteBehavior.Cascade);

        actorRole.HasIndex("RoleId").HasDatabaseName($"IX_{tableName}_RoleId");
        actorRole.HasIndex("ActorId").HasDatabaseName($"IX_{tableName}_ActorId");

        return builder;
    }

    public static ModelBuilder DeploymentConfiguration(this ModelBuilder builder)
    {
        var tableName = "Deployments";
        var deployment = builder.Entity("Deployment");

        deployment.ToTable(tableName);

        deployment.Property<Guid>("Id").IsRequired();
        deployment.HasKey("Id");

        deployment.Property<string>("Name").HasColumnType(Text).IsRequired();
        deployment.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        deployment.Property<string>("Status").HasColumnType(Text).IsRequired();
        deployment.Property<string>("Spec").HasColumnType(Json).IsRequired();
        deployment.Property<Guid>("PlatformId").IsRequired();

        deployment.Property<DateTime?>("AutoUpdateState_LastCheckedAt").HasColumnType(Timestamp).HasDefaultValue(null).IsRequired(false);
        deployment.Property<string>("AutoUpdateState_Status").HasColumnType(Text).HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_CurrentDigest").HasColumnType(Text).HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_RemoteDigest").HasColumnType(Text).HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_LastError").HasColumnType(Text).HasMaxLength(2000).HasDefaultValue(null);

        deployment
            .AddReconcilableMember()
            .AddAuditedMemebers();

        deployment
           .HasOne("Platform")
           .WithMany()
           .HasForeignKey("PlatformId")
           .OnDelete(DeleteBehavior.Restrict);

        deployment.HasIndex("Name", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_Name_PlatformId");

        return builder;
    }

    public static ModelBuilder ImageConfiguration(this ModelBuilder builder)
    {
        var tableName = "Images";
        var image = builder.Entity("Image");

        image.ToTable(tableName);

        image.Property<Guid>("Id").IsRequired();
        image.HasKey("Id");

        image.Property<Guid>("PlatformId").IsRequired();
        image.Property<Guid?>("RegistryId");
        image.Property<string>("Name").HasColumnType(Text).IsRequired();
        image.Property<string>("Tags").HasColumnType(Json).IsRequired();
        image.Property<string>("DockerImageId").HasColumnType(Text).IsRequired();
        image.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        image.Property<DateTime?>("UpdatedAt").HasColumnType(Timestamp).HasDefaultValue(null);
        image.Property<int>("Containers").HasColumnType(Integer).HasDefaultValue(0);
        image.Property<double>("Size").HasColumnType(Double).HasDefaultValue(0);
        image.AddReconcilableMember();

        image
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        image
            .HasOne("Registry")
            .WithMany()
            .HasForeignKey("RegistryId")
            .OnDelete(DeleteBehavior.SetNull);

        image.HasIndex("PlatformId").HasDatabaseName($"IX_{tableName}_PlatformId");
        image.HasIndex("DockerImageId", "PlatformId").IsUnique()
            .HasDatabaseName($"IX_{tableName}_DockerImageId_PlatformId");

        return builder;
    }

    public static ModelBuilder ActivityEventConfiguration(this ModelBuilder builder)
    {
        var tableName = "ActivityEvents";
        var activityEvent = builder.Entity("ActivityEvent");

        activityEvent.ToTable(tableName);

        activityEvent.Property<Guid>("Id").IsRequired();
        activityEvent.HasKey("Id");

        activityEvent.Property<Guid?>("PlatformId").IsRequired(false);
        activityEvent.Property<Guid?>("ResourceId").IsRequired(false);
        activityEvent.Property<string>("ResourceName").HasColumnType(Text).IsRequired();
        activityEvent.Property<string>("ResourceType").HasColumnType(Text).IsRequired();
        activityEvent.Property<string>("Status").HasColumnType(Text).IsRequired();
        activityEvent.Property<string>("EventType").HasColumnType(Text).IsRequired();
        activityEvent.Property<string>("Info").HasColumnType(Text).IsRequired();
        activityEvent.AddAuditedMemebers();

        activityEvent
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        activityEvent.HasIndex("PlatformId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Platform_CreatedAt");
        activityEvent.HasIndex("ResourceId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Resource_CreatedAt");
        activityEvent.HasIndex("EventType").HasDatabaseName($"IX_{tableName}_EventType");
        activityEvent.HasIndex("Status").HasDatabaseName($"IX_{tableName}_Status");

        return builder;
    }

    public static ModelBuilder AlertRuleConfiguration(this ModelBuilder builder)
    {
        var tableName = "AlertRules";
        var alertRule = builder.Entity("AlertRule");
        alertRule.ToTable(tableName);

        alertRule.Property<Guid>("Id").IsRequired();
        alertRule.HasKey("Id");

        alertRule.Property<string>("Type").HasColumnType(Text).IsRequired();
        alertRule.Property<string>("Name").HasColumnType(Text).HasMaxLength(120).IsRequired();
        alertRule.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        alertRule.Property<int?>("CooldownSeconds").HasColumnType(Integer).IsRequired(false);
        alertRule.Property<string>("Status").HasColumnType(Text).IsRequired().HasDefaultValue("Enabled");
        alertRule.Property<string>("LimitedTo").HasColumnType(Json).IsRequired();
        alertRule.Property<string>("QuietHours").HasColumnType(Json).IsRequired();
        alertRule.Property<int?>("RequiredMatches").HasColumnType(Integer).IsRequired(false);
        alertRule.Property<double?>("Threshold").HasColumnType(Double).IsRequired(false);
        alertRule.Property<string>("Severity").HasColumnType(Text).IsRequired();

        alertRule.AddAuditedMemebers();

        alertRule.HasIndex("Type").HasDatabaseName($"IX_{tableName}_Type");

        return builder;
    }

    public static ModelBuilder AlertRuleStateConfiguration(this ModelBuilder builder)
    {
        var tableName = "AlertRuleStates";
        var alertRuleState = builder.Entity("AlertRuleState");
        alertRuleState.ToTable(tableName);

        alertRuleState.Property<Guid?>("ResourceId").IsRequired(false);
        alertRuleState.Property<Guid>("AlertRuleId").IsRequired();
        alertRuleState.Property<DateTime?>("LastTriggeredAt").HasColumnType(Timestamp).HasDefaultValue(null);
        alertRuleState.Property<int>("ConsecutiveMatches").HasColumnType(Integer).IsRequired().HasDefaultValue(3);

        alertRuleState.HasKey("AlertRuleId", "ResourceId");

        alertRuleState
            .HasOne("AlertRule")
            .WithMany()
            .HasForeignKey("AlertRuleId")
            .OnDelete(DeleteBehavior.Cascade);

        alertRuleState.AddAuditedMemebers();
        return builder;
    }

    public static ModelBuilder AlertChannelConfiguration(this ModelBuilder builder)
    {
        var tableName = "AlertChannels";
        var alertChannel = builder.Entity("AlertChannel");
        alertChannel.ToTable(tableName);

        alertChannel.Property<Guid>("Id").IsRequired();
        alertChannel.HasKey("Id");

        alertChannel.Property<string>("AlertDestination").HasColumnType(Text).IsRequired();
        alertChannel.Property<string>("Url").HasColumnType(Text).IsRequired();
        alertChannel.Property<string>("Name").HasColumnType(Text).IsRequired();

        alertChannel.Property<bool>("IsActive").HasColumnType("boolean").IsRequired().HasDefaultValue(true);

        alertChannel.AddAuditedMemebers();
        return builder;
    }

    public static ModelBuilder StackConfiguration(this ModelBuilder builder)
    {
        var tableName = "Stacks";
        var stack = builder.Entity("Stack");

        stack.ToTable(tableName);

        stack.Property<Guid>("Id").IsRequired();
        stack.HasKey("Id");

        stack.Property<Guid?>("CurrentStackReleaseId").IsRequired(false);
        stack.Property<string>("Name").HasColumnType(Text).IsRequired();
        stack.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        stack.Property<string>("StackSource").HasColumnType(Text).IsRequired();
        stack.Property<string>("StackUpdateState").HasColumnType(Json).IsRequired();

        stack
            .AddReconcilableMember()
            .AddAuditedMemebers();

        // Uniqueness will be enforced at the application level since stacks can be shared across platforms and may have the same name
        // stack.HasIndex("Name", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_Name_PlatformId");
        stack.HasIndex("CurrentStackReleaseId").HasDatabaseName($"IX_{tableName}_CurrentStackReleaseId");

        return builder;
    }

    public static ModelBuilder StackReleaseConfiguration(this ModelBuilder builder)
    {
        var tableName = "StackReleases";
        var release = builder.Entity("StackRelease");

        release.ToTable(tableName);

        release.Property<Guid>("Id").IsRequired();
        release.HasKey("Id");

        release.Property<Guid>("StackId").IsRequired();
        release.Property<Guid>("PlatformId").IsRequired();
        release.Property<string>("Status").HasColumnType(Text).IsRequired();
        release.Property<string>("Version").HasColumnType(Text).IsRequired();
        release.Property<string>("Spec").HasColumnType(Json).IsRequired();

        release.AddAuditedMemebers();

        release
            .HasOne("Stack")
            .WithMany()
            .HasForeignKey("StackId")
            .OnDelete(DeleteBehavior.Cascade);

        release
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Restrict);

        release.HasIndex("StackId").HasDatabaseName($"IX_{tableName}_StackId");
        release.HasIndex("PlatformId").HasDatabaseName($"IX_{tableName}_PlatformId");

        return builder;
    }

    public static ModelBuilder AlertRuleChannelConfiguration(this ModelBuilder builder)
    {
        var tableName = "AlertRuleChannels";
        var alertRuleChannel = builder.Entity("AlertRuleChannel");
        alertRuleChannel.ToTable(tableName);

        alertRuleChannel.Property<Guid>("AlertRuleId").IsRequired();
        alertRuleChannel.Property<Guid>("AlertChannelId").IsRequired();
        alertRuleChannel.HasKey("AlertRuleId", "AlertChannelId");

        alertRuleChannel
            .HasOne("AlertRule")
            .WithMany()
            .HasForeignKey("AlertRuleId")
            .OnDelete(DeleteBehavior.Cascade);

        alertRuleChannel
            .HasOne("AlertChannel")
            .WithMany()
            .HasForeignKey("AlertChannelId")
            .OnDelete(DeleteBehavior.Cascade);

        alertRuleChannel.HasIndex("AlertChannelId").HasDatabaseName($"IX_{tableName}_AlertChannelId");

        return builder;
    }

    public static ModelBuilder AlertEventConfiguration(this ModelBuilder builder)
    {
        var tableName = "AlertEvents";
        var alertEvent = builder.Entity("AlertEvent");
        alertEvent.ToTable(tableName);

        alertEvent.Property<Guid>("Id").IsRequired();
        alertEvent.HasKey("Id");

        alertEvent.Property<Guid?>("ResourceId").IsRequired(false);
        alertEvent.Property<Guid>("AlertRuleId").IsRequired();
        alertEvent.Property<string>("Type").HasColumnType(Text).IsRequired();
        alertEvent.Property<string>("Severity").HasColumnType(Text).IsRequired();
        alertEvent.Property<string>("Info").HasColumnType(Json).IsRequired();
        alertEvent.Property<string>("ResourceType").HasColumnType(Text).IsRequired();
        alertEvent.Property<string>("DeduplicationKey").HasColumnType(Text).IsRequired();
        alertEvent.Property<string>("OpenIncidentKey").HasColumnType(Text).IsRequired(false);
        alertEvent.Property<Guid?>("AcknowledgedByActorId").IsRequired(false);
        alertEvent.Property<DateTime?>("AcknowledgedAt").HasColumnType(Timestamp).IsRequired(false);
        alertEvent.Property<Guid?>("ResolvedByActorId").IsRequired(false);
        alertEvent.Property<DateTime?>("ResolvedAt").HasColumnType(Timestamp).IsRequired(false);
        alertEvent.Property<string>("ResolutionNote").HasColumnType(Text).IsRequired(false);
        alertEvent.Property<string>("ResourceName").HasColumnType(Text).IsRequired();
        alertEvent.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        alertEvent.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        alertEvent
            .HasOne("AlertRule")
            .WithMany()
            .HasForeignKey("AlertRuleId")
            .OnDelete(DeleteBehavior.Cascade);

        alertEvent.HasIndex("Type").HasDatabaseName($"IX_{tableName}_Type");
        alertEvent.HasIndex("ResourceId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Resource_CreatedAt");
        alertEvent.HasIndex("ResourceType").HasDatabaseName($"IX_{tableName}_ResourceType");
        alertEvent.HasIndex("OpenIncidentKey").IsUnique().HasDatabaseName($"IX_{tableName}_OpenIncidentKey");

        return builder;
    }

    private static EntityTypeBuilder AddReconcilableMember(this EntityTypeBuilder builder)
    {
        builder.Property<long>("RowVersion").HasColumnType(BigInt).HasDefaultValue(0L);
        builder.Property<long?>("ControlStartedAt").HasColumnType(BigInt).HasDefaultValue(null);
        builder.Property<Guid?>("ControlTriggeredBy").IsRequired(false);
        builder.Property<string>("ControlState").HasColumnType(Text).HasMaxLength(64).HasDefaultValue(ResourceControlState.Idle.ToString());

        builder
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("ControlTriggeredBy")
            .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }

    private static EntityTypeBuilder AddAuditedMemebers(this EntityTypeBuilder builder)
    {
        builder.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        builder.Property<Guid>("CreatedByActorId").IsRequired();

        builder
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("CreatedByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }
}
