using Domain;
using Hosting.Common;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;
using Microsoft.EntityFrameworkCore.Storage.ValueConversion;

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
            .RefreshTokenConfiguration()
            .ActorConfiguration()
            .UserConfiguration()
            .TeamConfiguration()
            .PermissionConfiguration()
            .RoleConfiguration()
            .UserTeamConfiguration()
            .DeploymentConfiguration()
            .ImageConfiguration()
            .ActivityEventConfiguration();

        SeedDb(modelBuilder);
    }

    private static void SeedDb(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity("Role").HasData(new
        {
            Id = Guid.Parse("bdde9601-3b03-1275-a11b-98533d063a04"),
            Name = "Admin",
            CreatedAt = DateTime.Parse("2026-01-01"),
            UpdatedAt = DateTime.Parse("2026-01-01 00:00:00")
        });

        // --- Actors ---
        modelBuilder.Entity("Actor").HasData(new
        {
            Id = Constants.DefaultAdminId,
            Type = ActorType.User.ToString(),
            Name = "Admin"
        }, new 
        {
            Id = Constants.SystemId,
            Type = ActorType.System.ToString(),
            Name = "System"
        });

        // --- Users ---
        modelBuilder.Entity("User").HasData(new
        {
            Id = Guid.Parse("d1de9601-f113-ce77-884e-3cb636ec09a8"),
            ActorId = Constants.DefaultAdminId,
            Name = "admin",
            Email = "admin@admin.com",
            Password = "o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1",
            CreatedAt = DateTime.Parse("2026-01-01"),
            CreatedByActorId = Constants.SystemId
        });

        // --- Teams ---
        modelBuilder.Entity("Team").HasData(new
        {
            Id = Guid.Parse("cede9601-67e9-507d-832c-0ca0155465a1"),
            Name = "Admins",
            RoleId = Guid.Parse("bdde9601-3b03-1275-a11b-98533d063a04")
        });

        // --- UsersTeams ---
        modelBuilder.Entity("UserTeam").HasData(
            new
            {
                UserId = Guid.Parse("d1de9601-f113-ce77-884e-3cb636ec09a8"),
                TeamId = Guid.Parse("cede9601-67e9-507d-832c-0ca0155465a1")
            }
        );

        // --- Registries ---
        modelBuilder.Entity("Registry").HasData(new
        {
            Id = Constants.DefaultRegistryId,
            Name = Constants.DefaultRegistryName,
            Description = "Public Docker Hub Registry",
            RegistryHost = "hub.docker.com",
            CreatedByActorId = Constants.SystemId,
            CreatedAt = DateTime.Parse("2026-01-01"),
            Status = "Active",
            Configuration = """
            {
                "$type": "DockerHub"
            }
            """
        });
    }
}

internal static class Configuration
{
    public static ModelBuilder ContainerConfiguration(this ModelBuilder builder)
    {
        var tableName = "Containers";
        var container = builder.Entity("Container");

        container.ToTable(tableName);

        container.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        container.HasKey("Id");

        container.Property<Guid>("PlatformId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        container.Property<Guid?>("DeploymentId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired(false);
        container.Property<Guid?>("ImageId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired(false);
        container.Property<string>("DockerContainerId").HasColumnType("TEXT").IsRequired().HasMaxLength(64);
        container.Property<string>("DockerImageId").HasColumnType("TEXT").IsRequired();
        container.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        container.Property<long>("Created").HasColumnType("REAL").IsRequired();
        container.Property<string>("Updated").HasColumnType("TEXT").IsRequired();
        container.Property<string>("State").HasColumnType("TEXT").IsRequired();
        container.Property<string>("Stack").HasColumnType("TEXT");
        container.Property<string>("Ports").HasColumnType("TEXT").IsRequired();

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

    public static ModelBuilder PlatformConfiguration(this ModelBuilder builder)
    {
        var tableName = "Platforms";
        var platform = builder.Entity("Platform");

        platform.ToTable(tableName);

        platform.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        platform.HasKey("Id");

        platform.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        platform.Property<string>("Address").HasColumnType("TEXT").IsRequired();
        platform.Property<string>("Status").HasColumnType("TEXT").IsRequired();
        platform.Property<string>("ConnectorType").HasColumnType("TEXT").IsRequired();
        platform.Property<int>("NetworkCount").HasColumnType("REAL").IsRequired();
        platform.Property<int>("VolumeCount").HasColumnType("REAL").IsRequired();
        platform.Property<int>("ImageCount").HasColumnType("REAL").IsRequired();
        platform.Property<int>("CpuCount").HasColumnType("REAL").IsRequired();
        platform.Property<long>("MemTotal").HasColumnType("REAL").IsRequired();
        platform.Property<string>("AgentVersion").HasColumnType("TEXT");
        platform.Property<string>("ServerVersion").HasColumnType("TEXT");
        platform.Property<string>("PlatformDescriptor").IsRequired();

        platform.HasIndex("Address").IsUnique().HasDatabaseName($"IX_{tableName}_Address");

        return builder;
    }

    public static ModelBuilder ContainerStatConfiguration(this ModelBuilder builder)
    {
        var tableName = "ContainerStats";
        var stat = builder.Entity("ContainerStat");

        stat.ToTable(tableName);

        stat.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        stat.HasKey("Id");

        stat.Property<Guid>("ContainerId").HasConversion(GuidConverter).HasColumnType("TEXT").IsRequired();
        stat.Property<long>("Created").HasColumnType("REAL").IsRequired();
        stat.Property<double>("MemoryActive").HasColumnType("REAL");
        stat.Property<double>("MemoryCache").HasColumnType("REAL");
        stat.Property<double>("CpuUsage").HasColumnType("REAL");
        stat.Property<double>("MemoryLimit").HasColumnType("REAL");
        stat.Property<double>("RxBytes").HasColumnType("REAL");
        stat.Property<double>("TxBytes").HasColumnType("REAL");

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

        stat.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        stat.HasKey("Id");

        stat.Property<Guid>("PlatformId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        stat.Property<long>("Created").HasColumnType("REAL").IsRequired();
        stat.Property<double>("MemoryUsage").HasColumnType("REAL").IsRequired();
        stat.Property<double>("CpuUsage").HasColumnType("REAL").IsRequired();
        stat.Property<double>("RxBytes").HasColumnType("REAL").IsRequired();
        stat.Property<double>("TxBytes").HasColumnType("REAL").IsRequired();

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

        registry.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        registry.HasKey("Id");

        registry.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        registry.Property<string>("Description").HasColumnType("TEXT").IsRequired(false).HasMaxLength(600);
        registry.Property<string>("RegistryHost").HasColumnType("TEXT").IsRequired();
        registry.Property<string>("Status").HasColumnType("TEXT").IsRequired();
        registry.Property<string>("Configuration").HasColumnType("TEXT").IsRequired();

        registry.AddAuditedMemebers();

        registry.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");

        return builder;
    }

    public static ModelBuilder RefreshTokenConfiguration(this ModelBuilder builder)
    {
        var tableName = "RefreshTokens";
        var refreshToken = builder.Entity("RefreshToken");

        refreshToken.ToTable(tableName);

        refreshToken.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        refreshToken.HasKey("Id");

        refreshToken.Property<Guid>("UserId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        refreshToken.Property<string>("CreatedAt").HasColumnType("TEXT").IsRequired();

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

        actor.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        actor.HasKey("Id");

        actor.Property<string>("Type").IsRequired();
        actor.Property<string>("Name").HasColumnType("TEXT").IsRequired();

        return builder;
    }

    public static ModelBuilder UserConfiguration(this ModelBuilder builder)
    {
        var tableName = "Users";
        var user = builder.Entity("User");

        user.ToTable(tableName);

        user.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        user.HasKey("Id");

        user.Property<Guid>("ActorId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        user.Property<string>("Name").HasColumnType("TEXT").HasMaxLength(100).IsRequired();
        user.Property<string>("Email").HasColumnType("TEXT").IsRequired(false);
        user.Property<string>("Password").HasColumnType("TEXT").IsRequired(false);
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

        team.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        team.HasKey("Id");

        team.Property<Guid>("RoleId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        team.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        team
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("RoleId")
            .OnDelete(DeleteBehavior.Cascade);

        team.HasIndex("RoleId").HasDatabaseName($"IX_{tableName}_RoleId");

        return builder;
    }

    public static ModelBuilder PermissionConfiguration(this ModelBuilder builder)
    {
        var tableName = "Permissions";
        var permission = builder.Entity("Permission");

        permission.ToTable(tableName);
        
        permission.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        permission.HasKey("Id");
        
        permission.Property<Guid>("RoleId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        permission.Property<string>("PermissionCode").HasColumnType("TEXT").IsRequired();

        permission
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("RoleId")
            .OnDelete(DeleteBehavior.Cascade);

        permission.HasIndex("RoleId").HasDatabaseName($"IX_{tableName}_RoleId");

        return builder;
    }

    public static ModelBuilder RoleConfiguration(this ModelBuilder builder)
    {
        var tableName = "Roles";
        var role = builder.Entity("Role");

        role.ToTable(tableName);
        
        role.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        role.HasKey("Id");

        role.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        role.Property<DateTime>("CreatedAt").HasColumnType("TEXT").IsRequired().HasDefaultValue("2000-01-01 00:00:00");
        role.Property<DateTime>("UpdatedAt").HasColumnType("TEXT").IsRequired().HasDefaultValue("2000-01-01 00:00:00");

        return builder;
    }

    public static ModelBuilder UserTeamConfiguration(this ModelBuilder builder)
    {
        var tableName = "UsersTeams";
        var userTeam = builder.Entity("UserTeam");

        userTeam.ToTable(tableName);

        userTeam.Property<Guid>("UserId").HasColumnType("TEXT").IsRequired();
        userTeam.Property<Guid>("TeamId").HasColumnType("TEXT").IsRequired();
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

    public static ModelBuilder DeploymentConfiguration(this ModelBuilder builder)
    {
        var tableName = "Deployments";
        var deployment = builder.Entity("Deployment");

        deployment.ToTable(tableName);

        deployment.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        deployment.HasKey("Id");

        deployment.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        deployment.Property<string>("Description").HasColumnType("TEXT").IsRequired(false);
        deployment.Property<string>("Status").HasColumnType("TEXT").IsRequired();
        deployment.Property<string>("Spec").HasColumnType("TEXT").IsRequired();
        deployment.Property<Guid>("PlatformId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();

        deployment.Property<string>("UpdateBehavior").HasColumnType("TEXT").HasDefaultValue(null);
        deployment.Property<DateTime?>("AutoUpdateState_LastCheckedAt").HasColumnType("TEXT").HasDefaultValue(null).IsRequired(false);
        deployment.Property<string>("AutoUpdateState_Status").HasColumnType("TEXT").HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_CurrentDigest").HasColumnType("TEXT").HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_RemoteDigest").HasColumnType("TEXT").HasDefaultValue(null);
        deployment.Property<string>("AutoUpdateState_LastError").HasColumnType("TEXT").HasMaxLength(2000).HasDefaultValue(null);

        deployment
            .AddReconcilableMember()
            .AddAuditedMemebers();

        deployment
           .HasOne("Platform")
           .WithMany()
           .HasForeignKey("PlatformId")
           .OnDelete(DeleteBehavior.Cascade);

        deployment.HasIndex("Name", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_Name_PlatformId");

        return builder;
    }

    public static ModelBuilder ImageConfiguration(this ModelBuilder builder)
    {
        var tableName = "Images";
        var image = builder.Entity("Image");

        image.ToTable(tableName);

        image.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        image.HasKey("Id");

        image.Property<Guid>("PlatformId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        image.Property<Guid?>("RegistryId").HasColumnType("TEXT").HasConversion(GuidConverter);
        image.Property<string>("Name").HasColumnType("TEXT").IsRequired();
        image.Property<string>("Tags").HasColumnType("TEXT").IsRequired();
        image.Property<string>("DockerImageId").HasColumnType("TEXT").IsRequired();
        image.Property<string>("CreatedAt").HasColumnType("TEXT").IsRequired();
        image.Property<string?>("UpdatedAt").HasColumnType("TEXT").HasDefaultValue(null);
        image.Property<int>("Containers").HasColumnType("INTEGER").HasDefaultValue(0);
        image.Property<double>("Size").HasColumnType("REAL").HasDefaultValue(0);
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

        activityEvent.Property<Guid>("Id").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        activityEvent.HasKey("Id");

        activityEvent.Property<Guid>("PlatformId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        activityEvent.Property<Guid>("ResourceId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();
        activityEvent.Property<string>("ResourceName").HasColumnType("TEXT").IsRequired();
        activityEvent.Property<string>("ResourceType").HasColumnType("TEXT").IsRequired();
        activityEvent.Property<string>("EventType").HasColumnType("TEXT").IsRequired();
        activityEvent.Property<string>("Info").HasColumnType("TEXT").IsRequired();
        activityEvent.AddAuditedMemebers();

        activityEvent
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        activityEvent.HasIndex("PlatformId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Platform_CreatedAt");
        activityEvent.HasIndex("ResourceId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Resource_CreatedAt");
        activityEvent.HasIndex("EventType").HasDatabaseName($"IX_{tableName}_EventType");

        return builder;
    }

    private static EntityTypeBuilder AddReconcilableMember(this EntityTypeBuilder builder)
    {
        builder.Property<long>("RowVersion").HasColumnType("INTEGER").HasDefaultValue(0);
        builder.Property<long?>("ControlStartedAt").HasColumnType("INTEGER").HasDefaultValue(null);
        builder.Property<string>("ControlState").HasColumnType("TEXT").HasMaxLength(64).HasDefaultValue(ResourceControlState.Idle);

        return builder;
    }

    private static EntityTypeBuilder AddAuditedMemebers(this EntityTypeBuilder builder)
    {
        builder.Property<DateTime>("CreatedAt").HasColumnType("TEXT").IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        builder.Property<Guid>("CreatedByActorId").HasColumnType("TEXT").HasConversion(GuidConverter).IsRequired();

        builder
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("CreatedByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }

    private static readonly ValueConverter<Guid, string> GuidConverter = new(
       g => g.ToString("D").ToLowerInvariant(),
       s => Guid.Parse(s)
    );    
}
