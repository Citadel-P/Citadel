using Domain;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;
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
        modelBuilder.HasPostgresExtension("pg_trgm");

        modelBuilder
            .ContainerConfiguration()
            .ContainerStatConfiguration()
            .SwarmServiceStatConfiguration()
            .PlatformConfiguration()
            .SwarmNodeProjectionConfiguration()
            .SwarmServiceProjectionConfiguration()
            .SwarmTaskProjectionConfiguration()
            .SwarmNetworkProjectionConfiguration()
            .SwarmSecretProjectionConfiguration()
            .SwarmConfigProjectionConfiguration()
            .SwarmNodeRuntimeProjectionStateConfiguration()
            .EdgeAgentEnrollmentConfiguration()
            .EdgeAgentBindingConfiguration()
            .SwarmNodeAgentInstallationConfiguration()
            .SwarmNodeAgentBootstrapConfiguration()
            .PlatformStatConfiguration()
            .RegistryConfiguration()
            .GitAccountConfiguration()
            .GitRepositoryConfiguration()
            .GitRepositoryRefConfiguration()
            .RefreshTokenConfiguration()
            .UserMfaSettingsConfiguration()
            .UserMfaRecoveryCodeConfiguration()
            .MfaSetupSessionConfiguration()
            .MfaChallengeConfiguration()
            .UserPreferencesConfiguration()
            .CitadelInstanceIdentityConfiguration()
            .InstanceSetupStateConfiguration()
            .InstalledLicenseConfiguration()
            .ActorConfiguration()
            .UserConfiguration()
            .TeamConfiguration()
            .UserTeamConfiguration()
            .PermissionConfiguration()
            .ResourceAccessConfiguration()
            .RoleConfiguration()
            .ActorRoleConfiguration()
            .ResourceBindingConfiguration()
            .TagConfiguration()
            .ResourceTagConfiguration()
            .OidcProviderConfiguration()
            .OidcLoginStateConfiguration()
            .OidcExternalLoginConfiguration()
            .SecretProviderConfiguration()
            .SecretDefinitionConfiguration()
            .InternalSecretValueConfiguration()
            .DeploymentConfiguration()
            .SwarmServiceConfiguration()
            .ImageConfiguration()
            .AutomationActionConfiguration()
            .ActionRunConfiguration()
            .BackupConfiguration()
            .BuildAgentPoolConfiguration()
            .BuildConfiguration()
            .ActivityEventConfiguration()
            .AlertRuleConfiguration()
            .AlertEventConfiguration()
            .AlertRuleStateConfiguration()
            .AlertChannelConfiguration()
            .StackConfiguration()
            .StackReleaseConfiguration()
            .StackSwarmNamespaceReservationConfiguration()
            .StackWebhookDeployQueueConfiguration()
            .StackReleaseVolumeBindingConfiguration()
            .StackReleaseSwarmResourceConfiguration()
            .AlertRuleChannelConfiguration();

        SeedDb(modelBuilder);
        ToLower(modelBuilder);
    }

    private static void SeedDb(ModelBuilder modelBuilder)
    {
        var seedDate = new DateTime(2026, 1, 1, 0, 0, 0, DateTimeKind.Utc);
        // Actors
        var systemActorId = Constants.SystemId;
        var teamActorId = Constants.TeamActorId;

        // Users / Teams
        var teamId = Guid.Parse("20000000-0000-0000-0000-000000000001");

        // Roles
        var adminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");
        var operatorRoleId = Guid.Parse("30000000-0000-0000-0000-000000000002");
        var viewerRoleId = Guid.Parse("30000000-0000-0000-0000-000000000003");

        // Tags
        var SystemTagId = Guid.Parse("40000000-0000-0000-0000-000000000001");
        var prodTagId = Guid.Parse("40000000-0000-0000-0000-000000000002");

        // Actors
        modelBuilder.Entity("Actor").HasData(
            new { Id = systemActorId, Type = "System", IsEnabled = true },
            new { Id = teamActorId, Type = "Team", IsEnabled = true }
        );

        modelBuilder.Entity("InstanceSetupState").HasData(new
        {
            Id = InstanceSetupState.SingletonId,
            InitializedAt = (DateTimeOffset?)null,
            InitialAdministratorActorId = (Guid?)null,
            CreatedAt = new DateTimeOffset(seedDate),
            UpdatedAt = new DateTimeOffset(seedDate)
        });

        // Team
        modelBuilder.Entity("Team").HasData(new
        {
            Id = teamId,
            Name = "Operators",
            ActorId = teamActorId
        });

        // Roles
        modelBuilder.Entity("Role").HasData(
            new { Id = adminRoleId, Name = "Admin", RoleType = RoleType.System.ToString() },
            new { Id = operatorRoleId, Name = "Operator", RoleType = RoleType.System.ToString() },
            new { Id = viewerRoleId, Name = "Viewer", RoleType = RoleType.System.ToString() }
        );

        // ActorRoles
        modelBuilder.Entity("ActorRole").HasData(
            // Team -> Operator role
            new { ActorId = teamActorId, RoleId = operatorRoleId }
        );

        // Permissions — only valid combinations from the matrix are seeded
        var permissions = new List<object>();
        var defaultNonAdminRoleResources = new HashSet<ResourceType>
        {
            ResourceType.Platform,
            ResourceType.Deployment,
            ResourceType.SwarmService,
            ResourceType.Stack,
            ResourceType.Registry,
            ResourceType.GitRepository,
            ResourceType.GitAccount,
            ResourceType.Tag,
            ResourceType.Volume,
            ResourceType.BackupPolicy,
            ResourceType.BackupRepository,
            ResourceType.AutomationAction,
            ResourceType.Build,
            ResourceType.BuildAgentPool
        };

        foreach (var (resource, capabilities) in PermissionMatrix.GetAll())
        {
            var specificPermissions = capabilities.SpecificPermissionMinimumLevels.Keys.OrderBy(x => x).ToArray();

            permissions.Add(new
            {
                Id = CreatePermissionSeedId(adminRoleId, resource, PermissionLevel.Execute, specificPermissions),
                RoleId = adminRoleId,
                ResourceType = (int)resource,
                PermissionLevel = (int)PermissionLevel.Execute,
                SpecificPermissions = Domain.Entities.Identity.Permission.ToSpecificPermissionsMask(specificPermissions),
                RoleType = RoleType.System.ToString()
            });

            if (!defaultNonAdminRoleResources.Contains(resource))
                continue;

            permissions.Add(new
            {
                Id = CreatePermissionSeedId(viewerRoleId, resource, PermissionLevel.Read, []),
                RoleId = viewerRoleId,
                ResourceType = (int)resource,
                PermissionLevel = (int)PermissionLevel.Read,
                SpecificPermissions = 0,
                RoleType = RoleType.System.ToString()
            });

            var operatorSpecificPermissions = capabilities.SpecificPermissionMinimumLevels
                .Where(x => x.Value <= PermissionLevel.Write)
                .Select(x => x.Key)
                .OrderBy(x => x)
                .ToArray();

            permissions.Add(new
            {
                Id = CreatePermissionSeedId(operatorRoleId, resource, PermissionLevel.Write, operatorSpecificPermissions),
                RoleId = operatorRoleId,
                ResourceType = (int)resource,
                PermissionLevel = (int)PermissionLevel.Write,
                SpecificPermissions = Domain.Entities.Identity.Permission.ToSpecificPermissionsMask(operatorSpecificPermissions),
                RoleType = RoleType.System.ToString()
            });
        }

        modelBuilder.Entity("Permission").HasData(permissions);

        // Tags
        modelBuilder.Entity("Tag").HasData(
            new
            {
                Id = SystemTagId,
                Name = "System",
                NormalizedName = "system",
                Color = "#6b21a8",
                CreatedByActorId = systemActorId,
                CreatedAt = seedDate,
                UpdatedAt = seedDate
            },
            new
            {
                Id = prodTagId,
                Name = "Prod",
                NormalizedName = "prod",
                Color = "#f87171",
                CreatedByActorId = systemActorId,
                CreatedAt = seedDate,
                UpdatedAt = seedDate
            });

        // --- Alert Rules ---
        modelBuilder.Entity("AlertRule").HasData(
            // Platform threshold alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000001"), Name = "CPU > 90% - Platform", Type = "PlatformCpuHigh", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)90.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000011"), Name = "CPU > 80% - Platform", Type = "PlatformCpuHigh", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)80.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000002"), Name = "RAM > 90% - Platform", Type = "PlatformRamHigh", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)90.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000022"), Name = "RAM > 80% - Platform", Type = "PlatformRamHigh", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)80.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000023"), Name = "Disk > 70% - Platform", Type = "PlatformDiskHigh", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)70.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000024"), Name = "Disk > 90% - Platform", Type = "PlatformDiskHigh", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)3, Threshold = (double?)90.0, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Platform event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000003"), Name = "Platform Unreachable", Type = "PlatformUnreachable", Severity = "Critical", CooldownSeconds = 600, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000004"), Name = "Platform Version Mismatch", Type = "PlatformVersionMismatch", Severity = "Warning", CooldownSeconds = 3600, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000005"), Name = "Unmanaged Container Created", Type = "UnmanagedContainerCreated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Deployment event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000006"), Name = "Image Update Available - Deployment", Type = "DeploymentImageUpdateAvailable", Severity = "Info", CooldownSeconds = 60 * 60 * 24, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000007"), Name = "Auto Deploy Failed - Deployment", Type = "DeploymentAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000008"), Name = "Deployment Auto Updated", Type = "DeploymentAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Swarm Service event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000001c"), Name = "Operation Failed - Swarm Service", Type = "SwarmServiceOperationFailed", Severity = "Critical", CooldownSeconds = (int?)null, Status = AlertRuleStatus.Enabled.ToString(), LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // DockerStack event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000009"), Name = "Image Update Available - Stack", Type = "StackImageUpdateAvailable", Severity = "Info", CooldownSeconds = 60 * 60 * 24, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000a"), Name = "Auto Deploy Failed - Stack", Type = "StackAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000b"), Name = "Stack Auto Updated", Type = "StackAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000c"), Name = "Stack Drift Detected", Type = "StackDriftDetected", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000014"), Name = "Stack Drift Auto Reconciled", Type = "StackDriftAutoReconciled", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000d"), Name = "Stack Service Auto Updated", Type = "StackServiceAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000e"), Name = "Auto Deploy Failed - Stack Service", Type = "StackServiceAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000015"), Name = "Git Update Available - Stack", Type = "StackGitUpdateAvailable", Severity = "Info", CooldownSeconds = 60 * 60 * 24, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000016"), Name = "Git Stack Auto Updated", Type = "StackGitAutoUpdated", Severity = "Info", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000017"), Name = "Git Auto Deploy Failed - Stack", Type = "StackGitAutoDeployFailed", Severity = "Critical", CooldownSeconds = (int?)null, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Webhook event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000000f"), Name = "Webhook Authentication Failed", Type = "WebhookAuthenticationFailed", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000010"), Name = "Webhook Dispatch Failed", Type = "WebhookDispatchFailed", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000012"), Name = "Webhook Sync Failed - Git Repository", Type = "WebhookGitRepoSyncFailed", Severity = "Warning", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000013"), Name = "Webhook Deploy Failed - Git Stack", Type = "WebhookStackGitDeployFailed", Severity = "Critical", CooldownSeconds = 300, IsEnabled = true, Scope = "All", LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Automation event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000018"), Name = "Automation Action Run Failed", Type = "AutomationActionRunFailed", Severity = "Critical", CooldownSeconds = (int?)null, Status = AlertRuleStatus.Enabled.ToString(), LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // Build event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000001b"), Name = "Build Run Failed", Type = "BuildRunFailed", Severity = "Critical", CooldownSeconds = (int?)null, Status = AlertRuleStatus.Enabled.ToString(), LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },

            // License event alerts
            new { Id = Guid.Parse("019d0000-0001-7000-8001-000000000019"), Name = "License Entered Grace Period", Type = "LicenseEnteredGracePeriod", Severity = "Warning", CooldownSeconds = (int?)null, Status = AlertRuleStatus.Enabled.ToString(), LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate },
            new { Id = Guid.Parse("019d0000-0001-7000-8001-00000000001a"), Name = "License Expired", Type = "LicenseExpired", Severity = "Critical", CooldownSeconds = (int?)null, Status = AlertRuleStatus.Enabled.ToString(), LimitedTo = "[]", QuietHours = "[]", RequiredMatches = (int?)null, Threshold = (double?)null, CreatedByActorId = Constants.SystemId, CreatedAt = seedDate }
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

    private static Guid CreatePermissionSeedId(
        Guid roleId,
        ResourceType resourceType,
        PermissionLevel permissionLevel,
        IEnumerable<SpecificPermission> specificPermissions)
    {
        var input = $"permission:{roleId:D}:{resourceType}:{permissionLevel}:{string.Join(',', specificPermissions.OrderBy(x => x))}";
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
    private const string JsonB = "jsonb";
    private const string Integer = "integer";
    private const string BigInt = "bigint";
    private const string Double = "double precision";
    private const string Timestamp = "timestamp with time zone";

    private static void ConfigureGlobalSearchIndex(
        EntityTypeBuilder entity,
        string tableName,
        string propertyName,
        bool activeOnly = false)
    {
        var indexName = $"IX_{tableName}_GlobalSearch_{propertyName}_Trgm";
        var index = entity
            .HasIndex([propertyName], indexName)
            .HasDatabaseName(indexName)
            .HasMethod("gin")
            .HasOperators("gin_trgm_ops");

        if (activeOnly)
            index.HasFilter("archivedat IS NULL");
    }

    public static ModelBuilder CitadelInstanceIdentityConfiguration(this ModelBuilder builder)
    {
        var tableName = "CitadelInstanceIdentity";
        var identity = builder.Entity("CitadelInstanceIdentity");

        identity.ToTable(tableName);

        identity.Property<int>("Id").HasColumnType(Integer).IsRequired();
        identity.HasKey("Id");
        identity.HasCheckConstraint($"CK_{tableName}_Singleton", "\"id\" = 1");

        identity.Property<Guid>("InstanceId").IsRequired();
        identity.Property<DateTimeOffset>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        identity.HasIndex("InstanceId").IsUnique().HasDatabaseName($"IX_{tableName}_InstanceId");

        return builder;
    }

    public static ModelBuilder InstanceSetupStateConfiguration(this ModelBuilder builder)
    {
        var tableName = "InstanceSetupStates";
        var setup = builder.Entity("InstanceSetupState");

        setup.ToTable(tableName);

        setup.Property<short>("Id").HasColumnType("smallint").IsRequired();
        setup.HasKey("Id");
        setup.HasCheckConstraint($"CK_{tableName}_Singleton", "\"id\" = 1");

        setup.Property<DateTimeOffset?>("InitializedAt").HasColumnType(Timestamp).IsRequired(false);
        setup.Property<Guid?>("InitialAdministratorActorId").IsRequired(false);
        setup.Property<DateTimeOffset>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        setup.Property<DateTimeOffset>("UpdatedAt").HasColumnType(Timestamp).IsRequired();

        setup.HasCheckConstraint(
            $"CK_{tableName}_State",
            "(initializedat IS NULL AND initialadministratoractorid IS NULL) OR "
            + "(initializedat IS NOT NULL AND initialadministratoractorid IS NOT NULL)");

        setup
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("InitialAdministratorActorId")
            .OnDelete(DeleteBehavior.Restrict);

        return builder;
    }

    public static ModelBuilder InstalledLicenseConfiguration(this ModelBuilder builder)
    {
        var tableName = "InstalledLicenses";
        var license = builder.Entity("InstalledLicense");

        license.ToTable(tableName);

        license.Property<int>("Id").HasColumnType(Integer).IsRequired();
        license.HasKey("Id");
        license.HasCheckConstraint($"CK_{tableName}_Singleton", "\"id\" = 1");

        license.Property<string>("RawLicense").HasColumnType(Text).IsRequired();
        license.Property<string>("Fingerprint").HasColumnType(Text).IsRequired();
        license.Property<DateTimeOffset>("InstalledAt").HasColumnType(Timestamp).IsRequired();
        license.Property<Guid?>("InstalledByActorId").IsRequired(false);
        license.Property<DateTimeOffset?>("LastValidatedAt").HasColumnType(Timestamp).IsRequired(false);
        license.Property<string>("LastValidationStatus").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        license.Property<string>("LastValidationErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);

        license
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("InstalledByActorId")
            .OnDelete(DeleteBehavior.SetNull);

        return builder;
    }

    public static ModelBuilder ContainerConfiguration(this ModelBuilder builder)
    {
        var tableName = "Containers";
        var container = builder.Entity("Container");

        container.ToTable(tableName);

        container.Property<Guid>("Id").IsRequired();
        container.HasKey("Id");

        container.Property<Guid>("PlatformId").IsRequired();
        container.Property<Guid?>("DeploymentId").IsRequired(false);
        container.Property<Guid?>("StackId").IsRequired(false);
        container.Property<Guid?>("ImageId").IsRequired(false);
        container.Property<string>("DockerContainerId").HasColumnType(Text).IsRequired().HasMaxLength(64);
        container.Property<string>("DockerImageId").HasColumnType(Text).IsRequired();
        container.Property<string>("Name").HasColumnType(Text).IsRequired();
        container.Property<long>("Created").HasColumnType(BigInt).IsRequired();
        container.Property<long>("Updated").HasColumnType(BigInt).IsRequired();
        container.Property<string>("State").HasColumnType(Text).IsRequired();
        container.Property<string>("Stack").HasColumnType(Text);
        container.Property<bool>("IsSystem").IsRequired().HasDefaultValue(false);
        container.Property<string>("SystemRole").HasColumnType(Text).IsRequired(false);
        container.Property<bool>("HasCitadelOwnershipLabels").IsRequired().HasDefaultValue(false);
        container.Property<bool>("IsSwarmTask").IsRequired().HasDefaultValue(false);
        container.Property<string>("DockerNodeId").HasColumnType(Text).IsRequired(false).HasMaxLength(64);
        container.Property<long?>("ProjectionObservedAt").HasColumnType(BigInt).IsRequired(false);
        container.Property<long?>("ProjectionStaleSince").HasColumnType(BigInt).IsRequired(false);
        container.Property<string>("ProjectionStaleReason").HasColumnType(Text).IsRequired(false).HasMaxLength(256);
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

        container
            .HasOne("Stack")
            .WithMany()
            .IsRequired(false)
            .HasForeignKey("StackId")
            .OnDelete(DeleteBehavior.SetNull);

        container.HasIndex("DockerContainerId", "PlatformId")
            .IsUnique()
            .HasFilter("dockernodeid IS NULL")
            .HasDatabaseName($"IX__{tableName}_DockerContainerId_PlatformId");
        container.HasIndex("DockerContainerId", "PlatformId", "DockerNodeId")
            .IsUnique()
            .HasFilter("dockernodeid IS NOT NULL")
            .HasDatabaseName($"IX__{tableName}_DockerContainerId_PlatformId_DockerNodeId");
        container.HasIndex("PlatformId", "DockerNodeId").HasDatabaseName($"IX_{tableName}_PlatformId_DockerNodeId");
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
        gitRepository.Property<string>("SyncMode").HasColumnType(Text).IsRequired().HasDefaultValue("PullInterval");
        gitRepository.Property<int?>("SyncIntervalMinutes").HasColumnType(Integer).HasDefaultValue(5).IsRequired(false);
        gitRepository.Property<Guid?>("GitAccountId").IsRequired(false);
        gitRepository.Property<string>("Webhook").HasColumnType("jsonb").IsRequired(false);
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
        ConfigureGlobalSearchIndex(gitRepository, tableName, "Name");
        ConfigureGlobalSearchIndex(gitRepository, tableName, "Url");

        return builder;
    }

    public static ModelBuilder GitRepositoryRefConfiguration(this ModelBuilder builder)
    {
        var tableName = "GitRepositoryRefs";
        var gitRepositoryRef = builder.Entity("GitRepositoryRef");

        gitRepositoryRef.ToTable(tableName);

        gitRepositoryRef.Property<Guid>("Id").IsRequired();
        gitRepositoryRef.HasKey("Id");

        gitRepositoryRef.Property<Guid>("GitRepositoryId").IsRequired();
        gitRepositoryRef.Property<string>("Branch").HasColumnType(Text).IsRequired();
        gitRepositoryRef.Property<string>("ResolvedCommitSha").HasColumnType(Text).IsRequired(false);
        gitRepositoryRef.Property<string>("Status").HasColumnType(Text).IsRequired();
        gitRepositoryRef.Property<string>("LastError").HasColumnType(Text).IsRequired(false);
        gitRepositoryRef.Property<DateTime>("LastSyncedAt").HasColumnType(Timestamp).IsRequired();

        gitRepositoryRef
            .HasOne("GitRepository")
            .WithMany()
            .HasForeignKey("GitRepositoryId")
            .OnDelete(DeleteBehavior.Cascade);

        gitRepositoryRef.HasIndex("GitRepositoryId", "Branch").IsUnique()
            .HasDatabaseName($"IX_{tableName}_GitRepositoryId_Branch");

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
        platform.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
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
        platform.Property<string>("ClusterId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        platform.Property<bool>("PruneHistoricalSwarmTaskContainers").HasColumnType("boolean").IsRequired().HasDefaultValue(true);

        platform.HasIndex("Address").IsUnique().HasDatabaseName($"IX_{tableName}_Address");
        platform.HasIndex("ClusterId")
            .IsUnique()
            .HasFilter("clusterid IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_ClusterId");
        ConfigureGlobalSearchIndex(platform, tableName, "Name");
        ConfigureGlobalSearchIndex(platform, tableName, "Address");

        return builder;
    }

    public static ModelBuilder EdgeAgentEnrollmentConfiguration(this ModelBuilder builder)
    {
        var tableName = "EdgeAgentEnrollments";
        var enrollment = builder.Entity("EdgeAgentEnrollment");

        enrollment.ToTable(tableName);

        enrollment.Property<Guid>("Id").IsRequired();
        enrollment.HasKey("Id");

        enrollment.Property<Guid>("PlatformId").IsRequired();
        enrollment.Property<string>("ResourceType").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue("Platform");
        enrollment.Property<Guid>("ResourceId").IsRequired();
        enrollment.Property<string>("TokenHash").HasColumnType(Text).HasMaxLength(128).IsRequired();
        enrollment.Property<DateTime>("ExpiresAtUtc").HasColumnType(Timestamp).IsRequired();
        enrollment.Property<DateTime?>("UsedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        enrollment.Property<DateTime?>("RevokedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        enrollment.Property<Guid>("CreatedByActorId").IsRequired();
        enrollment.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        enrollment
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("CreatedByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        enrollment.HasIndex("PlatformId", "ExpiresAtUtc").HasDatabaseName($"IX_{tableName}_PlatformId_ExpiresAtUtc");
        enrollment.HasIndex("ResourceType", "ResourceId", "ExpiresAtUtc").HasDatabaseName($"IX_{tableName}_Resource_ExpiresAtUtc");
        enrollment.HasIndex("TokenHash").IsUnique().HasDatabaseName($"IX_{tableName}_TokenHash");

        return builder;
    }

    public static ModelBuilder EdgeAgentBindingConfiguration(this ModelBuilder builder)
    {
        var tableName = "EdgeAgentBindings";
        var binding = builder.Entity("EdgeAgentBinding");

        binding.ToTable(tableName);

        binding.Property<Guid>("Id").IsRequired();
        binding.HasKey("Id");

        binding.Property<Guid>("PlatformId").IsRequired();
        binding.Property<string>("ResourceType").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue("Platform");
        binding.Property<Guid>("ResourceId").IsRequired();
        binding.Property<Guid>("AgentId").IsRequired();
        binding.Property<string>("AgentPublicKey").HasColumnType(Text).IsRequired();
        binding.Property<string>("AgentFingerprint").HasColumnType(Text).HasMaxLength(128).IsRequired();
        binding.Property<string>("ConnectionStatus").HasColumnType(Text).HasMaxLength(64).IsRequired();
        binding.Property<DateTime?>("LastConnectedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<DateTime?>("LastDisconnectedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<DateTime?>("LastHeartbeatAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<string>("LastSeenVersion").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<string>("LastSeenHostname").HasColumnType(Text).HasMaxLength(256).IsRequired(false);
        binding.Property<string>("CapabilitiesJson").HasColumnType(Json).IsRequired(false);
        binding.Property<int>("ProtocolVersion").HasColumnType(Integer).IsRequired().HasDefaultValue(1);
        binding.Property<string>("Profile").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue("Ordinary");
        binding.Property<string>("ClusterId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<string>("DockerNodeId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<string>("DockerDaemonId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<string>("DockerHostname").HasColumnType(Text).HasMaxLength(256).IsRequired(false);
        binding.Property<string>("SwarmRole").HasColumnType(Text).HasMaxLength(32).IsRequired(false);
        binding.Property<string>("LastObservedServiceId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<string>("LastObservedTaskId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        binding.Property<DateTime?>("FirstEnrolledAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<DateTime?>("LastAuthenticatedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<string>("RevocationReason").HasColumnType(Text).HasMaxLength(512).IsRequired(false);
        binding.Property<DateTime?>("RevokedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        binding.Property<DateTime>("UpdatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        binding.HasIndex("ResourceType", "ResourceId")
            .IsUnique()
            .HasFilter("dockernodeid IS NULL AND revokedatutc IS NULL")
            .HasDatabaseName($"IX_{tableName}_ActiveResource");
        binding.HasIndex("ResourceType", "ResourceId", "DockerNodeId")
            .IsUnique()
            .HasFilter("dockernodeid IS NOT NULL AND revokedatutc IS NULL")
            .HasDatabaseName($"IX_{tableName}_ActiveNode");
        binding.HasIndex("DockerDaemonId")
            .IsUnique()
            .HasFilter("dockerdaemonid IS NOT NULL AND revokedatutc IS NULL")
            .HasDatabaseName($"IX_{tableName}_ActiveDaemon");
        binding.HasIndex("AgentId")
            .IsUnique()
            .HasFilter("revokedatutc IS NULL")
            .HasDatabaseName($"IX_{tableName}_ActiveAgentId");
        binding.HasIndex("AgentFingerprint")
            .IsUnique()
            .HasFilter("revokedatutc IS NULL")
            .HasDatabaseName($"IX_{tableName}_ActiveAgentFingerprint");

        return builder;
    }

    public static ModelBuilder SwarmNodeAgentInstallationConfiguration(this ModelBuilder builder)
    {
        var tableName = "SwarmNodeAgentInstallations";
        var installation = builder.Entity("SwarmNodeAgentInstallation");

        installation.ToTable(tableName);
        installation.Property<Guid>("PlatformId").IsRequired();
        installation.HasKey("PlatformId");
        installation.Property<string>("ClusterId").HasColumnType(Text).HasMaxLength(128).IsRequired();
        installation.Property<string>("ManagerDockerNodeId").HasColumnType(Text).HasMaxLength(128).IsRequired();
        installation.Property<string>("ManagerDockerDaemonId").HasColumnType(Text).HasMaxLength(128).IsRequired();
        installation.Property<string>("DockerServiceId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        installation.Property<string>("DockerServiceName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        installation.Property<string>("AgentImageReference").HasColumnType(Text).HasMaxLength(512).IsRequired();
        installation.Property<string>("AgentImageDigest").HasColumnType(Text).HasMaxLength(512).IsRequired();
        installation.Property<string>("DockerCaConfigId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        installation.Property<string>("DockerCaConfigName").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        installation.Property<string>("DesiredState").HasColumnType(Text).HasMaxLength(32).IsRequired();
        installation.Property<Guid?>("OperationId").IsRequired(false);
        installation.Property<string>("OperationKind").HasColumnType(Text).HasMaxLength(32).IsRequired(false);
        installation.Property<string>("OperationState").HasColumnType(Text).HasMaxLength(32).IsRequired(false);
        installation.Property<DateTime?>("OperationStartedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        installation.Property<Guid?>("OperationActorId").IsRequired(false);
        installation.Property<string>("OperationError").HasColumnType(Text).HasMaxLength(2000).IsRequired(false);
        installation.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        installation.Property<DateTime>("UpdatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        installation.HasOne("Platform").WithOne().HasForeignKey("SwarmNodeAgentInstallation", "PlatformId").OnDelete(DeleteBehavior.Cascade);
        installation.HasOne("Actor").WithMany().HasForeignKey("OperationActorId").OnDelete(DeleteBehavior.Restrict);
        installation.HasIndex("DockerServiceId").IsUnique().HasFilter("dockerserviceid IS NOT NULL").HasDatabaseName($"IX_{tableName}_DockerServiceId");
        return builder;
    }

    public static ModelBuilder SwarmNodeAgentBootstrapConfiguration(this ModelBuilder builder)
    {
        var tableName = "SwarmNodeAgentBootstraps";
        var bootstrap = builder.Entity("SwarmNodeAgentBootstrap");

        bootstrap.ToTable(tableName);
        bootstrap.Property<Guid>("Id").IsRequired();
        bootstrap.HasKey("Id");
        bootstrap.Property<Guid>("PlatformId").IsRequired();
        bootstrap.Property<string>("ClusterId").HasColumnType(Text).HasMaxLength(128).IsRequired();
        bootstrap.Property<int>("Version").HasColumnType(Integer).IsRequired();
        bootstrap.Property<string>("TokenHash").HasColumnType(Text).HasMaxLength(128).IsRequired();
        bootstrap.Property<string>("DockerSecretId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        bootstrap.Property<string>("DockerSecretName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        bootstrap.Property<DateTime>("ExpiresAtUtc").HasColumnType(Timestamp).IsRequired();
        bootstrap.Property<DateTime?>("RevokedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        bootstrap.Property<Guid>("CreatedByActorId").IsRequired();
        bootstrap.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        bootstrap.Property<DateTime>("UpdatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        bootstrap.HasOne("Platform").WithMany().HasForeignKey("PlatformId").OnDelete(DeleteBehavior.Cascade);
        bootstrap.HasOne("Actor").WithMany().HasForeignKey("CreatedByActorId").OnDelete(DeleteBehavior.Restrict);
        bootstrap.HasIndex("PlatformId", "Version").IsUnique().HasDatabaseName($"IX_{tableName}_PlatformVersion");
        bootstrap.HasIndex("TokenHash").IsUnique().HasDatabaseName($"IX_{tableName}_TokenHash");
        bootstrap.HasIndex("DockerSecretId").IsUnique().HasFilter("dockersecretid IS NOT NULL").HasDatabaseName($"IX_{tableName}_DockerSecretId");
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
        stat.HasIndex("Created")
            .HasDatabaseName($"IX_{tableName}_Created");

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
        stat.Property<long?>("DiskUsedBytes").HasColumnType(BigInt).IsRequired(false);
        stat.Property<long?>("DiskTotalBytes").HasColumnType(BigInt).IsRequired(false);
        stat.Property<double?>("DiskUsage").HasColumnType(Double).IsRequired(false);

        stat
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        stat.HasIndex("PlatformId", "Created").IsUnique()
            .HasDatabaseName($"IX_{tableName}_PlatformId_Created");
        stat.HasIndex("Created")
            .HasDatabaseName($"IX_{tableName}_Created");

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
        ConfigureGlobalSearchIndex(registry, tableName, "Name");
        ConfigureGlobalSearchIndex(registry, tableName, "RegistryHost");

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
        refreshToken.Property<DateTime>("LastSeenAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        refreshToken.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        refreshToken.Property<string>("UserAgent").HasColumnType(Text).IsRequired(false);
        refreshToken.Property<string>("IpAddress").HasColumnType(Text).IsRequired(false);

        refreshToken
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        refreshToken.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");
        refreshToken.HasIndex("ExpiresAt").HasDatabaseName($"IX_{tableName}_ExpiresAt");

        return builder;
    }

    public static ModelBuilder UserMfaSettingsConfiguration(this ModelBuilder builder)
    {
        var tableName = "UserMfaSettings";
        var settings = builder.Entity("UserMfaSettings");

        settings.ToTable(tableName);

        settings.Property<Guid>("UserId").IsRequired();
        settings.HasKey("UserId");

        settings.Property<string>("ProtectedTotpSecret").HasColumnType(Text).IsRequired();
        settings.Property<long?>("LastAcceptedTimeStep").HasColumnType("bigint").IsRequired(false);
        settings.Property<DateTime>("EnabledAt").HasColumnType(Timestamp).IsRequired();
        settings.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        settings
            .HasOne("User")
            .WithOne()
            .HasForeignKey("UserMfaSettings", "UserId")
            .OnDelete(DeleteBehavior.Cascade);

        return builder;
    }

    public static ModelBuilder UserMfaRecoveryCodeConfiguration(this ModelBuilder builder)
    {
        var tableName = "UserMfaRecoveryCodes";
        var code = builder.Entity("UserMfaRecoveryCode");

        code.ToTable(tableName);

        code.Property<Guid>("Id").IsRequired();
        code.HasKey("Id");

        code.Property<Guid>("UserId").IsRequired();
        code.Property<string>("CodeHash").HasColumnType(Text).IsRequired();
        code.Property<DateTime?>("UsedAt").HasColumnType(Timestamp).IsRequired(false);
        code.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        code
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        code.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");
        code.HasIndex("UserId", "CodeHash").IsUnique().HasDatabaseName($"IX_{tableName}_UserId_CodeHash");

        return builder;
    }

    public static ModelBuilder MfaSetupSessionConfiguration(this ModelBuilder builder)
    {
        var tableName = "MfaSetupSessions";
        var session = builder.Entity("MfaSetupSession");

        session.ToTable(tableName);

        session.Property<Guid>("Id").IsRequired();
        session.HasKey("Id");

        session.Property<Guid>("UserId").IsRequired();
        session.Property<string>("ProtectedTotpSecret").HasColumnType(Text).IsRequired();
        session.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired();
        session.Property<DateTime?>("ConsumedAt").HasColumnType(Timestamp).IsRequired(false);
        session.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        session
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        session.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");
        session.HasIndex("ExpiresAt").HasDatabaseName($"IX_{tableName}_ExpiresAt");

        return builder;
    }

    public static ModelBuilder MfaChallengeConfiguration(this ModelBuilder builder)
    {
        var tableName = "MfaChallenges";
        var challenge = builder.Entity("MfaChallenge");

        challenge.ToTable(tableName);

        challenge.Property<Guid>("Id").IsRequired();
        challenge.HasKey("Id");

        challenge.Property<Guid>("UserId").IsRequired();
        challenge.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired();
        challenge.Property<int>("FailedAttempts").HasColumnType("integer").IsRequired().HasDefaultValue(0);
        challenge.Property<DateTime?>("ConsumedAt").HasColumnType(Timestamp).IsRequired(false);
        challenge.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();

        challenge
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        challenge.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");
        challenge.HasIndex("ExpiresAt").HasDatabaseName($"IX_{tableName}_ExpiresAt");

        return builder;
    }

    public static ModelBuilder UserPreferencesConfiguration(this ModelBuilder builder)
    {
        var tableName = "UserPreferences";
        var preferences = builder.Entity("UserPreferences");

        preferences.ToTable(tableName);

        preferences.Property<Guid>("UserId").IsRequired();
        preferences.HasKey("UserId");

        preferences.Property<string>("TimeZone").HasColumnType(Text).IsRequired();
        preferences.Property<string>("DateTimeFormat").HasColumnType(Text).IsRequired();
        preferences.Property<string>("Theme").HasColumnType(Text).IsRequired();
        preferences.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired();

        preferences
            .HasOne("User")
            .WithOne()
            .HasForeignKey("UserPreferences", "UserId")
            .OnDelete(DeleteBehavior.Cascade);

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
        permission.Property<int>("ResourceType").HasColumnType(Integer).IsRequired();
        permission.Property<int>("PermissionLevel").HasColumnType(Integer).IsRequired();
        permission.Property<int>("SpecificPermissions").HasColumnType(Integer).IsRequired();

        permission
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("RoleId")
            .OnDelete(DeleteBehavior.Cascade);

        permission.HasIndex("RoleId").HasDatabaseName($"IX_{tableName}_RoleId");
        permission.HasIndex("RoleId", "ResourceType").IsUnique().HasDatabaseName($"IX_{tableName}_RoleId_ResourceType");

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
        resourceAccess.Property<int>("ResourceType").HasColumnType(Integer).IsRequired();
        resourceAccess.Property<int>("PermissionLevel").HasColumnType(Integer).IsRequired();
        resourceAccess.Property<int>("SpecificPermissions").HasColumnType(Integer).IsRequired();

        resourceAccess.HasIndex(
            "ResourceType",
            "ResourceId",
            "ActorId")
        .IsUnique();

        resourceAccess.HasIndex(
            "ResourceType",
            "ResourceId",
            "ActorId",
            "PermissionLevel")
        .HasDatabaseName($"IX_{tableName}_PermissionLookup");

        resourceAccess.HasIndex(
            "ActorId",
            "ResourceType")
        .HasDatabaseName($"IX_{tableName}_Actor_ResourceType");

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
        role.Property<string>("RoleType").HasColumnType(Text).IsRequired();

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

    public static ModelBuilder ResourceBindingConfiguration(this ModelBuilder builder)
    {
        var tableName = "ResourceBindings";
        var entry = builder.Entity("ResourceBinding");

        entry.ToTable(tableName);

        entry.Property<Guid>("Id").IsRequired();
        entry.HasKey("Id");

        entry.Property<string>("Name").HasColumnType(Text).IsRequired();
        entry.Property<string>("Kind").HasColumnType(Text).IsRequired();
        entry.Property<string>("Scope").HasColumnType(Text).IsRequired();
        entry.Property<Guid?>("ResourceId").IsRequired(false);
        entry.Property<string>("Value").HasColumnType(Text).IsRequired(false);
        entry.Property<Guid?>("SecretId").IsRequired(false);
        entry.Property<string>("SecretDeliveryMode").HasColumnType(Text).IsRequired(false);
        entry.Property<string>("TargetPath").HasColumnType(Text).IsRequired(false);
        entry.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        entry.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        entry
            .HasOne("SecretDefinition")
            .WithMany()
            .HasForeignKey("SecretId")
            .OnDelete(DeleteBehavior.Restrict);

        entry.HasIndex("Scope", "Name")
            .IsUnique()
            .HasFilter("resourceid IS NULL")
            .HasDatabaseName($"IX_{tableName}_Scope_Name_Global");
        entry.HasIndex("Scope", "ResourceId", "Name")
            .IsUnique()
            .HasFilter("resourceid IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_Scope_ResourceId_Name");
        entry.HasIndex("SecretId").HasDatabaseName($"IX_{tableName}_SecretId");

        return builder;
    }

    public static ModelBuilder SecretDefinitionConfiguration(this ModelBuilder builder)
    {
        var tableName = "SecretDefinitions";
        var secret = builder.Entity("SecretDefinition");

        secret.ToTable(tableName);

        secret.Property<Guid>("Id").IsRequired();
        secret.HasKey("Id");

        secret.Property<string>("Name").HasColumnType(Text).IsRequired();
        secret.Property<string>("ProviderType").HasColumnType(Text).IsRequired();
        secret.Property<Guid?>("ProviderId").IsRequired(false);
        secret.Property<string>("ExternalPath").HasColumnType(Text).IsRequired(false);
        secret.Property<string>("ExternalKey").HasColumnType(Text).IsRequired(false);
        secret.Property<int?>("ExternalVersion").HasColumnType(Integer).IsRequired(false);
        secret.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        secret.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        secret
            .HasOne("SecretProvider")
            .WithMany()
            .HasForeignKey("ProviderId")
            .OnDelete(DeleteBehavior.Restrict);

        secret.HasIndex("Name").HasDatabaseName($"IX_{tableName}_Name");
        secret.HasIndex("ProviderId").HasDatabaseName($"IX_{tableName}_ProviderId");

        return builder;
    }

    public static ModelBuilder SecretProviderConfiguration(this ModelBuilder builder)
    {
        var tableName = "SecretProviders";
        var provider = builder.Entity("SecretProvider");

        provider.ToTable(tableName);

        provider.Property<Guid>("Id").IsRequired();
        provider.HasKey("Id");

        provider.Property<string>("Name").HasColumnType(Text).IsRequired();
        provider.Property<string>("ProviderType").HasColumnType(Text).IsRequired();
        provider.Property<string>("Configuration").HasColumnType(Text).IsRequired();
        provider.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        provider.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        provider.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");

        return builder;
    }

    public static ModelBuilder OidcProviderConfiguration(this ModelBuilder builder)
    {
        var tableName = "OidcProviders";
        var provider = builder.Entity("OidcProvider");

        provider.ToTable(tableName);

        provider.Property<Guid>("Id").IsRequired();
        provider.HasKey("Id");

        provider.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        provider.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        provider.Property<string>("DisplayName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        provider.Property<string>("Issuer").HasColumnType(Text).HasMaxLength(512).IsRequired();
        provider.Property<string>("ClientId").HasColumnType(Text).HasMaxLength(256).IsRequired();
        provider.Property<string>("ClientSecretCiphertext").HasColumnType(Text).IsRequired(false);
        provider.Property<string>("Scopes").HasColumnType(Text).HasMaxLength(512).IsRequired();
        provider.Property<bool>("Enabled").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        provider.Property<bool>("AutoProvisionUsers").HasColumnType("boolean").IsRequired().HasDefaultValue(false);
        provider.Property<bool>("AllowEmailAutoLink").HasColumnType("boolean").IsRequired().HasDefaultValue(false);
        provider.Property<bool>("RequireEmailVerified").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        provider.Property<string>("AllowedEmailDomains").HasColumnType(Text).HasMaxLength(1024).IsRequired(false);
        provider.Property<string>("RequiredClaimName").HasColumnType(Text).HasMaxLength(256).IsRequired(false);
        provider.Property<string>("RequiredClaimValues").HasColumnType(Text).HasMaxLength(1024).IsRequired(false);
        provider.Property<Guid?>("DefaultRoleId").IsRequired(false);
        provider.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        provider.AddAuditedMemebers();

        provider
            .HasOne("Role")
            .WithMany()
            .HasForeignKey("DefaultRoleId")
            .OnDelete(DeleteBehavior.SetNull);

        provider.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");
        provider.HasIndex("Enabled").HasDatabaseName($"IX_{tableName}_Enabled");

        return builder;
    }

    public static ModelBuilder OidcLoginStateConfiguration(this ModelBuilder builder)
    {
        var tableName = "OidcLoginStates";
        var state = builder.Entity("OidcLoginState");

        state.ToTable(tableName);

        state.Property<Guid>("Id").IsRequired();
        state.HasKey("Id");

        state.Property<Guid>("ProviderId").IsRequired();
        state.Property<string>("StateHash").HasColumnType(Text).HasMaxLength(128).IsRequired();
        state.Property<string>("Nonce").HasColumnType(Text).HasMaxLength(256).IsRequired();
        state.Property<string>("CodeVerifier").HasColumnType(Text).HasMaxLength(256).IsRequired();
        state.Property<string>("ReturnUrl").HasColumnType(Text).HasMaxLength(2048).IsRequired();
        state.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        state.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired();

        state
            .HasOne("OidcProvider")
            .WithMany()
            .HasForeignKey("ProviderId")
            .OnDelete(DeleteBehavior.Cascade);

        state.HasIndex("StateHash").IsUnique().HasDatabaseName($"IX_{tableName}_StateHash");
        state.HasIndex("ProviderId").HasDatabaseName($"IX_{tableName}_ProviderId");
        state.HasIndex("ExpiresAt").HasDatabaseName($"IX_{tableName}_ExpiresAt");

        return builder;
    }

    public static ModelBuilder OidcExternalLoginConfiguration(this ModelBuilder builder)
    {
        var tableName = "OidcExternalLogins";
        var login = builder.Entity("OidcExternalLogin");

        login.ToTable(tableName);

        login.Property<Guid>("Id").IsRequired();
        login.HasKey("Id");

        login.Property<Guid>("ProviderId").IsRequired();
        login.Property<string>("Subject").HasColumnType(Text).HasMaxLength(512).IsRequired();
        login.Property<Guid>("UserId").IsRequired();
        login.Property<string>("Email").HasColumnType(Text).HasMaxLength(320).IsRequired(false);
        login.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        login.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        login
            .HasOne("OidcProvider")
            .WithMany()
            .HasForeignKey("ProviderId")
            .OnDelete(DeleteBehavior.Cascade);

        login
            .HasOne("User")
            .WithMany()
            .HasForeignKey("UserId")
            .OnDelete(DeleteBehavior.Cascade);

        login.HasIndex("ProviderId", "Subject").IsUnique().HasDatabaseName($"IX_{tableName}_ProviderId_Subject");
        login.HasIndex("UserId").HasDatabaseName($"IX_{tableName}_UserId");

        return builder;
    }

    public static ModelBuilder InternalSecretValueConfiguration(this ModelBuilder builder)
    {
        var tableName = "InternalSecretValues";
        var value = builder.Entity("InternalSecretValue");

        value.ToTable(tableName);

        value.Property<Guid>("SecretId").IsRequired();
        value.HasKey("SecretId");

        value.Property<string>("EncryptedValue").HasColumnType(Text).IsRequired();
        value.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        value.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        value
            .HasOne("SecretDefinition")
            .WithMany()
            .HasForeignKey("SecretId")
            .OnDelete(DeleteBehavior.Cascade);

        return builder;
    }

    public static ModelBuilder TagConfiguration(this ModelBuilder builder)
    {
        var tableName = "Tags";
        var tag = builder.Entity("Tag");

        tag.ToTable(tableName);

        tag.Property<Guid>("Id").IsRequired();
        tag.HasKey("Id");

        tag.Property<string>("Name").HasColumnType(Text).HasMaxLength(64).IsRequired();
        tag.Property<string>("NormalizedName").HasColumnType(Text).HasMaxLength(64).IsRequired();
        tag.Property<string>("Color").HasColumnType(Text).HasMaxLength(7).IsRequired();
        tag.AddAuditedMemebers();
        tag.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        tag.HasIndex("NormalizedName").IsUnique().HasDatabaseName($"IX_{tableName}_NormalizedName");

        return builder;
    }

    public static ModelBuilder ResourceTagConfiguration(this ModelBuilder builder)
    {
        var tableName = "ResourceTags";
        var resourceTag = builder.Entity("ResourceTag");

        resourceTag.ToTable(tableName);

        resourceTag.Property<string>("ResourceType").HasColumnType(Text).IsRequired();
        resourceTag.Property<Guid>("ResourceId").IsRequired();
        resourceTag.Property<Guid>("TagId").IsRequired();
        resourceTag.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        resourceTag.Property<Guid>("CreatedByActorId").IsRequired();

        resourceTag.HasKey("ResourceType", "ResourceId", "TagId");

        resourceTag
            .HasOne("Tag")
            .WithMany()
            .HasForeignKey("TagId")
            .OnDelete(DeleteBehavior.Cascade);

        resourceTag
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("CreatedByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        resourceTag.HasIndex("TagId").HasDatabaseName($"IX_{tableName}_TagId");
        resourceTag.HasIndex("ResourceType", "ResourceId").HasDatabaseName($"IX_{tableName}_Resource");
        resourceTag.HasIndex("ResourceType", "TagId", "ResourceId").HasDatabaseName($"IX_{tableName}_Filter");

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
        deployment.Property<string>("Spec").HasColumnType(JsonB).IsRequired();
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
        ConfigureGlobalSearchIndex(deployment, tableName, "Name");

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
            .OnDelete(DeleteBehavior.SetNull);

        activityEvent.HasIndex("PlatformId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Platform_CreatedAt");
        activityEvent.HasIndex("ResourceId", "CreatedAt").HasDatabaseName($"IX_{tableName}_Resource_CreatedAt");
        activityEvent.HasIndex("EventType").HasDatabaseName($"IX_{tableName}_EventType");
        activityEvent.HasIndex("Status").HasDatabaseName($"IX_{tableName}_Status");

        var latest = builder.Entity("LatestActivityEvent");
        latest.HasNoKey();
        latest.Property<Guid?>("ResourceId").HasColumnType("uuid").HasColumnName("resourceid");
        latest.Property<string>("ResourceType").HasColumnType(Text).HasColumnName("resourcetype");
        latest.Property<string>("Info").HasColumnType(Text).HasColumnName("info");
        latest.Property<DateTime>("CreatedAt").HasColumnType("timestamp with time zone").HasColumnName("createdat");
        latest.ToView("LatestActivityEvents");

        return builder;
    }

    public static ModelBuilder AutomationActionConfiguration(this ModelBuilder builder)
    {
        var tableName = "Actions";
        var action = builder.Entity("AutomationAction");

        action.ToTable(tableName);

        action.Property<Guid>("Id").IsRequired();
        action.HasKey("Id");

        action.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        action.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        action.Property<string>("Code").HasColumnType(Text).IsRequired();
        action.Property<string>("DefaultArgsJson").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'{}'::jsonb");
        action.Property<bool>("Enabled").HasColumnType("boolean").IsRequired();
        action.Property<bool>("ScheduleEnabled").HasColumnType("boolean").IsRequired();
        action.Property<string>("ScheduleCron").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        action.Property<string>("ScheduleTimeZone").HasColumnType(Text).HasMaxLength(128).IsRequired();
        action.Property<string>("Webhook").HasColumnType("jsonb").IsRequired(false);
        action.Property<int>("TimeoutSeconds").HasColumnType(Integer).IsRequired();
        action.Property<bool>("AlertOnFailure").HasColumnType("boolean").IsRequired();
        action.Property<Guid>("RunAsActorId").IsRequired();
        action.Property<DateTime?>("LastScheduledRunAt").HasColumnType(Timestamp).IsRequired(false);
        action.Property<Guid?>("CurrentRunId").IsRequired(false);
        action.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        action
            .AddReconcilableMember(includeControlTriggeredBy: false, requireControlState: true)
            .AddAuditedMemebers();

        action
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("RunAsActorId")
            .OnDelete(DeleteBehavior.Restrict);

        action.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");
        ConfigureGlobalSearchIndex(action, tableName, "Name");
        action.HasIndex("CreatedByActorId").HasDatabaseName($"IX_{tableName}_CreatedByActorId");
        action.HasIndex("RunAsActorId").HasDatabaseName($"IX_{tableName}_RunAsActorId");
        action.HasIndex("Enabled", "ScheduleEnabled", "ScheduleCron").HasDatabaseName($"IX_{tableName}_Schedule");
        action.HasIndex("ControlState", "ControlStartedAt").HasDatabaseName($"IX_{tableName}_ControlState_ControlStartedAt");

        return builder;
    }

    public static ModelBuilder ActionRunConfiguration(this ModelBuilder builder)
    {
        var tableName = "ActionRuns";
        var run = builder.Entity("ActionRun");

        run.ToTable(tableName);

        run.Property<Guid>("Id").IsRequired();
        run.HasKey("Id");

        run.Property<Guid>("ActionId").IsRequired();
        run.Property<string>("ActionName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        run.Property<string>("Trigger").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<Guid>("RunAsActorId").IsRequired();
        run.Property<Guid?>("TriggeredByActorId").IsRequired(false);
        run.Property<string>("ArgsJson").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'{}'::jsonb");
        run.Property<string>("CodeSnapshot").HasColumnType(Text).IsRequired();
        run.Property<string>("CodeHash").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<int>("TimeoutSeconds").HasColumnType(Integer).IsRequired();
        run.Property<DateTime>("QueuedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        run.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<DateTime?>("FinishedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<long?>("DurationMs").HasColumnType(BigInt).IsRequired(false);
        run.Property<int?>("ExitCode").HasColumnType(Integer).IsRequired(false);
        run.Property<string>("Logs").HasColumnType(Text).IsRequired(false);
        run.Property<string>("ErrorMessage").HasColumnType(Text).IsRequired(false);

        run
            .HasOne("AutomationAction")
            .WithMany()
            .HasForeignKey("ActionId")
            .OnDelete(DeleteBehavior.Cascade);

        run
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("RunAsActorId")
            .OnDelete(DeleteBehavior.Restrict);

        run
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("TriggeredByActorId")
            .OnDelete(DeleteBehavior.SetNull);

        run.HasIndex("ActionId", "QueuedAt", "Id")
            .IsDescending(false, true, true)
            .IncludeProperties("Status")
            .HasDatabaseName($"IX_{tableName}_ActionId_QueuedAt");
        run.HasIndex("ActionId")
            .IsUnique()
            .HasFilter("status IN ('Queued', 'Running')")
            .HasDatabaseName($"IX_{tableName}_Active_Action");
        run.HasIndex("Status", "QueuedAt").HasDatabaseName($"IX_{tableName}_Status_QueuedAt");
        run.HasIndex("RunAsActorId").HasDatabaseName($"IX_{tableName}_RunAsActorId");
        run.HasIndex("TriggeredByActorId").HasDatabaseName($"IX_{tableName}_TriggeredByActorId");

        return builder;
    }

    public static ModelBuilder BackupConfiguration(this ModelBuilder builder)
    {
        var repositoryTable = "BackupRepositories";
        var repository = builder.Entity("BackupRepository");
        repository.ToTable(repositoryTable);

        repository.Property<Guid>("Id").IsRequired();
        repository.HasKey("Id");
        repository.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        repository.Property<string>("NormalizedName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        repository.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        repository.Property<string>("Type").HasColumnType(Text).HasMaxLength(64).IsRequired();
        repository.Property<string>("Spec").HasColumnType("jsonb").IsRequired();
        repository.Property<Guid>("PasswordSecretId").IsRequired();
        repository.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        repository.Property<Guid?>("CurrentRunId").IsRequired(false);
        repository.Property<DateTime?>("LastPrunedAt").HasColumnType(Timestamp).IsRequired(false);
        repository.Property<DateTime?>("LastCheckedAt").HasColumnType(Timestamp).IsRequired(false);
        repository.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        repository.Property<DateTime?>("ArchivedAt").HasColumnType(Timestamp).IsRequired(false);
        repository
            .AddReconcilableMember(includeControlTriggeredBy: false, requireControlState: true)
            .AddAuditedMemebers();

        repository
            .HasOne("SecretDefinition")
            .WithMany()
            .HasForeignKey("PasswordSecretId")
            .OnDelete(DeleteBehavior.Restrict);

        repository.HasIndex("NormalizedName").IsUnique().HasDatabaseName($"IX_{repositoryTable}_NormalizedName");
        repository.HasIndex("Status").HasDatabaseName($"IX_{repositoryTable}_Status");
        repository.HasIndex("ControlState", "ControlStartedAt").HasDatabaseName($"IX_{repositoryTable}_ControlState_ControlStartedAt");
        repository.HasIndex("ArchivedAt").HasDatabaseName($"IX_{repositoryTable}_ArchivedAt");
        repository.HasIndex("PasswordSecretId").HasDatabaseName($"IX_{repositoryTable}_PasswordSecretId");
        ConfigureGlobalSearchIndex(repository, repositoryTable, "Name", activeOnly: true);
        ConfigureGlobalSearchIndex(repository, repositoryTable, "Type", activeOnly: true);

        var validationTable = "BackupRepositoryValidations";
        var validation = builder.Entity("BackupRepositoryValidation");
        validation.ToTable(validationTable);

        validation.Property<Guid>("Id").IsRequired();
        validation.HasKey("Id");
        validation.Property<Guid>("BackupRepositoryId").IsRequired();
        validation.Property<string>("Location").HasColumnType(Text).HasMaxLength(64).IsRequired();
        validation.Property<Guid?>("PlatformId").IsRequired(false);
        validation.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        validation.Property<DateTime>("LastValidatedAt").HasColumnType(Timestamp).IsRequired();
        validation.Property<string>("LastErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        validation.Property<string>("LastErrorMessage").HasColumnType(Text).HasMaxLength(600).IsRequired(false);

        validation
            .HasOne("BackupRepository")
            .WithMany()
            .HasForeignKey("BackupRepositoryId")
            .OnDelete(DeleteBehavior.Cascade);

        validation
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.SetNull);

        validation.HasIndex("BackupRepositoryId", "Location", "PlatformId").HasDatabaseName($"IX_{validationTable}_Repository_Location_Platform");
        validation.HasIndex("PlatformId").HasDatabaseName($"IX_{validationTable}_PlatformId");

        var policyTable = "BackupPolicies";
        var policy = builder.Entity("BackupPolicy");
        policy.ToTable(policyTable);

        policy.Property<Guid>("Id").IsRequired();
        policy.HasKey("Id");
        policy.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        policy.Property<string>("NormalizedName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        policy.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        policy.Property<string>("Source").HasColumnType("jsonb").IsRequired();
        policy.Property<Guid>("BackupRepositoryId").IsRequired();
        policy.Property<bool>("Enabled").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        policy.Property<string>("Cron").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        policy.Property<string>("TimeZone").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        policy.Property<string>("Webhook").HasColumnType("jsonb").IsRequired(false);
        policy.Property<int>("KeepLastSuccessful").HasColumnType(Integer).IsRequired().HasDefaultValue(14);
        policy.Property<int>("TimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(14400);
        policy.Property<bool>("AlertOnFailure").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        policy.Property<Guid>("RunAsActorId").IsRequired();
        policy.Property<Guid?>("CurrentRunId").IsRequired(false);
        policy.Property<DateTime?>("LastScheduledRunAt").HasColumnType(Timestamp).IsRequired(false);
        policy.Property<DateTime?>("FirstSuccessfulRunAt").HasColumnType(Timestamp).IsRequired(false);
        policy.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        policy.Property<DateTime?>("ArchivedAt").HasColumnType(Timestamp).IsRequired(false);
        policy
            .AddReconcilableMember(includeControlTriggeredBy: false, requireControlState: true)
            .AddAuditedMemebers();

        policy
            .HasOne("BackupRepository")
            .WithMany()
            .HasForeignKey("BackupRepositoryId")
            .OnDelete(DeleteBehavior.Restrict);

        policy
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("RunAsActorId")
            .OnDelete(DeleteBehavior.Restrict);

        policy.HasIndex("NormalizedName").IsUnique().HasDatabaseName($"IX_{policyTable}_NormalizedName");
        policy.HasIndex("BackupRepositoryId").HasDatabaseName($"IX_{policyTable}_BackupRepositoryId");
        policy.HasIndex("Enabled", "Cron").HasDatabaseName($"IX_{policyTable}_Schedule");
        policy.HasIndex("ControlState", "ControlStartedAt").HasDatabaseName($"IX_{policyTable}_ControlState_ControlStartedAt");
        policy.HasIndex("ArchivedAt").HasDatabaseName($"IX_{policyTable}_ArchivedAt");
        policy.HasIndex("RunAsActorId").HasDatabaseName($"IX_{policyTable}_RunAsActorId");
        policy.HasIndex("Source").HasMethod("gin").HasDatabaseName($"IX_{policyTable}_Source_Gin");
        ConfigureGlobalSearchIndex(policy, policyTable, "Name", activeOnly: true);

        var runTable = "BackupRuns";
        var run = builder.Entity("BackupRun");
        run.ToTable(runTable);

        run.Property<Guid>("Id").IsRequired();
        run.HasKey("Id");
        run.Property<Guid>("BackupPolicyId").IsRequired();
        run.Property<Guid>("BackupRepositoryId").IsRequired();
        run.Property<string>("PolicyNameSnapshot").HasColumnType(Text).HasMaxLength(128).IsRequired();
        run.Property<string>("SourceSnapshot").HasColumnType("jsonb").IsRequired();
        run.Property<string>("RepositoryTypeSnapshot").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<string>("Trigger").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<Guid?>("TriggerSourceId").IsRequired(false);
        run.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<string>("ResticSnapshotId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("ParentSnapshotId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("SnapshotAvailability").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<long?>("FilesProcessed").HasColumnType(BigInt).IsRequired(false);
        run.Property<long?>("BytesProcessed").HasColumnType(BigInt).IsRequired(false);
        run.Property<long?>("BytesAdded").HasColumnType(BigInt).IsRequired(false);
        run.Property<string>("Warnings").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        run.Property<DateTime>("QueuedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        run.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<DateTime?>("CompletedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<int?>("ExitCode").HasColumnType(Integer).IsRequired(false);
        run.Property<string>("ErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("ErrorMessage").HasColumnType(Text).HasMaxLength(1200).IsRequired(false);
        run.Property<Guid>("TriggeredByActorId").IsRequired();

        run
            .HasOne("BackupPolicy")
            .WithMany()
            .HasForeignKey("BackupPolicyId")
            .OnDelete(DeleteBehavior.Restrict);

        run
            .HasOne("BackupRepository")
            .WithMany()
            .HasForeignKey("BackupRepositoryId")
            .OnDelete(DeleteBehavior.Restrict);

        run
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("TriggeredByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        run.HasIndex("BackupPolicyId", "QueuedAt", "Id")
            .IsDescending(false, true, true)
            .IncludeProperties("Status")
            .HasDatabaseName($"IX_{runTable}_Policy_QueuedAt");
        run.HasIndex("BackupRepositoryId", "Status").HasDatabaseName($"IX_{runTable}_Repository_Status");
        run.HasIndex("BackupPolicyId")
            .IsUnique()
            .HasFilter("status IN ('Queued', 'Preparing', 'Running', 'ApplyingRetention')")
            .HasDatabaseName($"IX_{runTable}_Active_Policy");
        run.HasIndex("QueuedAt").HasDatabaseName($"IX_{runTable}_QueuedAt");
        run.HasIndex("Status", "QueuedAt").HasDatabaseName($"IX_{runTable}_Status_QueuedAt");
        run.HasIndex("SnapshotAvailability").HasDatabaseName($"IX_{runTable}_SnapshotAvailability");
        run.HasIndex("TriggeredByActorId").HasDatabaseName($"IX_{runTable}_TriggeredByActorId");

        var runItemTable = "BackupRunItems";
        var runItem = builder.Entity("BackupRunItem");
        runItem.ToTable(runItemTable);
        runItem.Property<Guid>("Id").IsRequired();
        runItem.HasKey("Id");
        runItem.Property<Guid>("BackupRunId").IsRequired();
        runItem.Property<Guid>("PlatformId").IsRequired();
        runItem.Property<string>("VolumeName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        runItem.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        runItem.Property<string>("ResticSnapshotId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        runItem.Property<string>("ParentSnapshotId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        runItem.Property<long?>("FilesProcessed").HasColumnType(BigInt).IsRequired(false);
        runItem.Property<long?>("BytesProcessed").HasColumnType(BigInt).IsRequired(false);
        runItem.Property<long?>("BytesAdded").HasColumnType(BigInt).IsRequired(false);
        runItem.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        runItem.Property<DateTime?>("CompletedAt").HasColumnType(Timestamp).IsRequired(false);
        runItem.Property<int?>("ExitCode").HasColumnType(Integer).IsRequired(false);
        runItem.Property<string>("ErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        runItem.Property<string>("ErrorMessage").HasColumnType(Text).HasMaxLength(1200).IsRequired(false);
        runItem.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        runItem.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        runItem
            .HasOne("BackupRun")
            .WithMany()
            .HasForeignKey("BackupRunId")
            .OnDelete(DeleteBehavior.Cascade);

        runItem
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        runItem.HasIndex("BackupRunId", "VolumeName").HasDatabaseName($"IX_{runItemTable}_Run_VolumeName");
        runItem.HasIndex("BackupRunId", "Status").HasDatabaseName($"IX_{runItemTable}_Run_Status");
        runItem.HasIndex("PlatformId", "VolumeName").HasDatabaseName($"IX_{runItemTable}_Platform_VolumeName");
        runItem.HasIndex("ResticSnapshotId").HasDatabaseName($"IX_{runItemTable}_ResticSnapshotId");

        var runLogTable = "BackupRunLogs";
        var runLog = builder.Entity("BackupRunLog");
        runLog.ToTable(runLogTable);
        runLog.Property<Guid>("Id").IsRequired();
        runLog.HasKey("Id");
        runLog.Property<Guid>("BackupRunId").IsRequired();
        runLog.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        runLog.Property<string>("Stream").HasColumnType(Text).HasMaxLength(32).IsRequired();
        runLog.Property<string>("Message").HasColumnType(Text).IsRequired();
        runLog
            .HasOne("BackupRun")
            .WithMany()
            .HasForeignKey("BackupRunId")
            .OnDelete(DeleteBehavior.Cascade);
        runLog.HasIndex("BackupRunId", "CreatedAt").HasDatabaseName($"IX_{runLogTable}_Run_CreatedAt");

        var restoreTable = "BackupRestoreRuns";
        var restore = builder.Entity("BackupRestoreRun");
        restore.ToTable(restoreTable);

        restore.Property<Guid>("Id").IsRequired();
        restore.HasKey("Id");
        restore.Property<Guid>("BackupRunId").IsRequired();
        restore.Property<Guid>("BackupRepositoryId").IsRequired();
        restore.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        restore.Property<Guid>("TargetPlatformId").IsRequired();
        restore.Property<string>("TargetVolumeName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        restore.Property<bool>("OverwriteExisting").HasColumnType("boolean").IsRequired();
        restore.Property<bool>("TargetVolumeCreatedByCitadel").HasColumnType("boolean").IsRequired();
        restore.Property<string>("AffectedContainers").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        restore.Property<string>("Warnings").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        restore.Property<DateTime>("QueuedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        restore.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        restore.Property<DateTime?>("CompletedAt").HasColumnType(Timestamp).IsRequired(false);
        restore.Property<int?>("ExitCode").HasColumnType(Integer).IsRequired(false);
        restore.Property<string>("ErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        restore.Property<string>("ErrorMessage").HasColumnType(Text).HasMaxLength(1200).IsRequired(false);
        restore.Property<Guid>("TriggeredByActorId").IsRequired();

        restore
            .HasOne("BackupRun")
            .WithMany()
            .HasForeignKey("BackupRunId")
            .OnDelete(DeleteBehavior.Restrict);

        restore
            .HasOne("BackupRepository")
            .WithMany()
            .HasForeignKey("BackupRepositoryId")
            .OnDelete(DeleteBehavior.Restrict);

        restore
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("TargetPlatformId")
            .OnDelete(DeleteBehavior.Restrict);

        restore
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("TriggeredByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        restore.HasIndex("BackupRunId", "QueuedAt").HasDatabaseName($"IX_{restoreTable}_BackupRun_QueuedAt");
        restore.HasIndex("BackupRepositoryId", "Status").HasDatabaseName($"IX_{restoreTable}_Repository_Status");
        restore.HasIndex("TargetPlatformId", "TargetVolumeName").HasDatabaseName($"IX_{restoreTable}_TargetVolume");
        restore.HasIndex("QueuedAt").HasDatabaseName($"IX_{restoreTable}_QueuedAt");
        restore.HasIndex("Status", "QueuedAt").HasDatabaseName($"IX_{restoreTable}_Status_QueuedAt");

        var restoreLogTable = "BackupRestoreRunLogs";
        var restoreLog = builder.Entity("BackupRestoreRunLog");
        restoreLog.ToTable(restoreLogTable);
        restoreLog.Property<Guid>("Id").IsRequired();
        restoreLog.HasKey("Id");
        restoreLog.Property<Guid>("BackupRestoreRunId").IsRequired();
        restoreLog.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        restoreLog.Property<string>("Stream").HasColumnType(Text).HasMaxLength(32).IsRequired();
        restoreLog.Property<string>("Message").HasColumnType(Text).IsRequired();
        restoreLog
            .HasOne("BackupRestoreRun")
            .WithMany()
            .HasForeignKey("BackupRestoreRunId")
            .OnDelete(DeleteBehavior.Cascade);
        restoreLog.HasIndex("BackupRestoreRunId", "CreatedAt").HasDatabaseName($"IX_{restoreLogTable}_RestoreRun_CreatedAt");

        var repositoryLeaseTable = "BackupRepositoryLeases";
        var repositoryLease = builder.Entity("BackupRepositoryLease");
        repositoryLease.ToTable(repositoryLeaseTable);
        repositoryLease.Property<Guid>("BackupRepositoryId").IsRequired();
        repositoryLease.HasKey("BackupRepositoryId");
        repositoryLease.Property<string>("OperationType").HasColumnType(Text).HasMaxLength(64).IsRequired();
        repositoryLease.Property<Guid>("OwnerRunId").IsRequired();
        repositoryLease.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired();
        repositoryLease.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        repositoryLease
            .HasOne("BackupRepository")
            .WithMany()
            .HasForeignKey("BackupRepositoryId")
            .OnDelete(DeleteBehavior.Cascade);
        repositoryLease.HasIndex("ExpiresAt").HasDatabaseName($"IX_{repositoryLeaseTable}_ExpiresAt");

        var sourceLeaseTable = "BackupSourceLeases";
        var sourceLease = builder.Entity("BackupSourceLease");
        sourceLease.ToTable(sourceLeaseTable);
        sourceLease.Property<string>("SourceKey").HasColumnType(Text).HasMaxLength(512).IsRequired();
        sourceLease.HasKey("SourceKey");
        sourceLease.Property<string>("OperationType").HasColumnType(Text).HasMaxLength(64).IsRequired();
        sourceLease.Property<Guid>("OwnerRunId").IsRequired();
        sourceLease.Property<DateTime>("ExpiresAt").HasColumnType(Timestamp).IsRequired();
        sourceLease.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        sourceLease.HasIndex("ExpiresAt").HasDatabaseName($"IX_{sourceLeaseTable}_ExpiresAt");

        return builder;
    }

    public static ModelBuilder BuildConfiguration(this ModelBuilder builder)
    {
        var projectTable = "BuildProjects";
        var project = builder.Entity("BuildProject");
        project.ToTable(projectTable);
        project.Property<Guid>("Id").IsRequired();
        project.HasKey("Id");
        project.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        project.Property<string>("NormalizedName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        project.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        project.Property<bool>("Enabled").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        project.Property<Guid>("GitRepositoryId").IsRequired();
        project.Property<string>("Branch").HasColumnType(Text).HasMaxLength(256).IsRequired();
        project.Property<string>("ContextPath").HasColumnType(Text).HasMaxLength(512).IsRequired().HasDefaultValue(".");
        project.Property<string>("DockerfilePath").HasColumnType(Text).HasMaxLength(512).IsRequired().HasDefaultValue("Dockerfile");
        project.Property<string>("Target").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        project.Property<string>("BuildArgs").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        project.Property<string>("BuildSecrets").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        project.Property<string>("BuilderKind").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue("Platform");
        project.Property<Guid?>("PlatformId").IsRequired(false);
        project.Property<Guid?>("BuildAgentPoolId").IsRequired(false);
        project.Property<Guid>("RegistryId").IsRequired();
        project.Property<string>("ImageRepository").HasColumnType(Text).HasMaxLength(512).IsRequired();
        project.Property<string>("TagTemplates").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[\"{branch}-{shortSha}\"]'::jsonb");
        project.Property<string>("Webhook").HasColumnType("jsonb").IsRequired(false);
        project.Property<int>("TimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(1800);
        project.Property<int>("RetentionRunCount").HasColumnType(Integer).IsRequired().HasDefaultValue(20);
        project.Property<Guid?>("CurrentRunId").IsRequired(false);
        project.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        project.Property<DateTime?>("ArchivedAt").HasColumnType(Timestamp).IsRequired(false);
        project
            .AddReconcilableMember(includeControlTriggeredBy: false, requireControlState: true)
            .AddAuditedMemebers();

        project.HasOne("GitRepository").WithMany().HasForeignKey("GitRepositoryId").OnDelete(DeleteBehavior.Restrict);
        project.HasOne("Platform").WithMany().HasForeignKey("PlatformId").OnDelete(DeleteBehavior.Restrict);
        project.HasOne("BuildAgentPool").WithMany().HasForeignKey("BuildAgentPoolId").OnDelete(DeleteBehavior.Restrict);
        project.HasOne("Registry").WithMany().HasForeignKey("RegistryId").OnDelete(DeleteBehavior.Restrict);

        project.HasIndex("NormalizedName").IsUnique().HasDatabaseName($"IX_{projectTable}_NormalizedName");
        project.HasIndex("GitRepositoryId").HasDatabaseName($"IX_{projectTable}_GitRepositoryId");
        project.HasIndex("BuilderKind").HasDatabaseName($"IX_{projectTable}_BuilderKind");
        project.HasIndex("PlatformId").HasDatabaseName($"IX_{projectTable}_PlatformId");
        project.HasIndex("BuildAgentPoolId").HasDatabaseName($"IX_{projectTable}_BuildAgentPoolId");
        project.HasIndex("RegistryId").HasDatabaseName($"IX_{projectTable}_RegistryId");
        project.HasIndex("ArchivedAt").HasDatabaseName($"IX_{projectTable}_ArchivedAt");
        project.HasIndex("ControlState", "ControlStartedAt").HasDatabaseName($"IX_{projectTable}_ControlState_ControlStartedAt");
        ConfigureGlobalSearchIndex(project, projectTable, "Name", activeOnly: true);
        ConfigureGlobalSearchIndex(project, projectTable, "Branch", activeOnly: true);
        project.ToTable(t => t.HasCheckConstraint(
            $"CK_{projectTable}_Builder_Target",
            "(\"builderkind\" = 'Platform' AND \"platformid\" IS NOT NULL AND \"buildagentpoolid\" IS NULL) OR (\"builderkind\" = 'BuildAgentPool' AND \"platformid\" IS NULL AND \"buildagentpoolid\" IS NOT NULL)"));

        var runTable = "BuildRuns";
        var run = builder.Entity("BuildRun");
        run.ToTable(runTable);
        run.Property<Guid>("Id").IsRequired();
        run.HasKey("Id");
        run.Property<Guid>("BuildProjectId").IsRequired();
        run.Property<string>("ProjectNameSnapshot").HasColumnType(Text).HasMaxLength(128).IsRequired();
        run.Property<Guid>("GitRepositoryId").IsRequired();
        run.Property<string>("GitRepositoryNameSnapshot").HasColumnType(Text).HasMaxLength(128).IsRequired();
        run.Property<string>("Branch").HasColumnType(Text).HasMaxLength(256).IsRequired();
        run.Property<string>("ResolvedCommitSha").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("ContextPath").HasColumnType(Text).HasMaxLength(512).IsRequired();
        run.Property<string>("DockerfilePath").HasColumnType(Text).HasMaxLength(512).IsRequired();
        run.Property<string>("Target").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("BuildArgsSnapshot").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        run.Property<string>("BuildSecretIdsSnapshot").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        run.Property<string>("PlatformSnapshot").HasColumnType("jsonb").IsRequired();
        run.Property<string>("RegistrySnapshot").HasColumnType("jsonb").IsRequired();
        run.Property<string>("ImageRepository").HasColumnType(Text).HasMaxLength(512).IsRequired();
        run.Property<string>("TagTemplatesSnapshot").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        run.Property<string>("ImageReferences").HasColumnType("jsonb").IsRequired().HasDefaultValueSql("'[]'::jsonb");
        run.Property<string>("Trigger").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<Guid?>("TriggerSourceId").IsRequired(false);
        run.Property<string>("Status").HasColumnType(Text).HasMaxLength(64).IsRequired();
        run.Property<string>("ImageDigest").HasColumnType(Text).HasMaxLength(256).IsRequired(false);
        run.Property<int>("TimeoutSeconds").HasColumnType(Integer).IsRequired();
        run.Property<DateTime>("QueuedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        run.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<DateTime?>("CompletedAt").HasColumnType(Timestamp).IsRequired(false);
        run.Property<int?>("ExitCode").HasColumnType(Integer).IsRequired(false);
        run.Property<string>("ErrorCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        run.Property<string>("ErrorMessage").HasColumnType(Text).HasMaxLength(1200).IsRequired(false);
        run.Property<Guid>("TriggeredByActorId").IsRequired();

        run.HasOne("BuildProject").WithMany().HasForeignKey("BuildProjectId").OnDelete(DeleteBehavior.Restrict);
        run.HasOne("GitRepository").WithMany().HasForeignKey("GitRepositoryId").OnDelete(DeleteBehavior.Restrict);
        run.HasOne("Actor").WithMany().HasForeignKey("TriggeredByActorId").OnDelete(DeleteBehavior.Restrict);
        run.HasIndex("BuildProjectId", "QueuedAt", "Id")
            .IsDescending(false, true, true)
            .IncludeProperties("Status")
            .HasDatabaseName($"IX_{runTable}_Project_QueuedAt");
        run.HasIndex("BuildProjectId").IsUnique().HasFilter("status IN ('Queued', 'Preparing', 'Running')").HasDatabaseName($"IX_{runTable}_Active_Project");
        run.HasIndex("Status", "QueuedAt").HasDatabaseName($"IX_{runTable}_Status_QueuedAt");
        run.HasIndex("QueuedAt").HasDatabaseName($"IX_{runTable}_QueuedAt");
        run.HasIndex("TriggeredByActorId").HasDatabaseName($"IX_{runTable}_TriggeredByActorId");

        var runLogTable = "BuildRunLogs";
        var runLog = builder.Entity("BuildRunLog");
        runLog.ToTable(runLogTable);
        runLog.Property<Guid>("Id").IsRequired();
        runLog.HasKey("Id");
        runLog.Property<Guid>("BuildRunId").IsRequired();
        runLog.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired();
        runLog.Property<string>("Stream").HasColumnType(Text).HasMaxLength(32).IsRequired();
        runLog.Property<string>("Message").HasColumnType(Text).IsRequired();
        runLog.HasOne("BuildRun").WithMany().HasForeignKey("BuildRunId").OnDelete(DeleteBehavior.Cascade);
        runLog.HasIndex("BuildRunId", "CreatedAt").HasDatabaseName($"IX_{runLogTable}_Run_CreatedAt");

        return builder;
    }

    public static ModelBuilder SwarmServiceStatConfiguration(this ModelBuilder builder)
    {
        var tableName = "SwarmServiceStats";
        var stat = builder.Entity("SwarmServiceStat");

        stat.ToTable(tableName);

        stat.Property<Guid>("Id").IsRequired();
        stat.HasKey("Id");

        stat.Property<Guid>("PlatformId").IsRequired();
        stat.Property<string>("DockerServiceId").HasColumnType(Text).HasMaxLength(255).IsRequired();
        stat.Property<Guid?>("SwarmServiceId").IsRequired(false);
        stat.Property<Guid?>("StackId").IsRequired(false);
        stat.Property<string>("ServiceName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        stat.Property<string>("TaskKey").HasColumnType(Text).HasMaxLength(512).IsRequired();
        stat.Property<string>("DockerTaskId").HasColumnType(Text).HasMaxLength(255).IsRequired();
        stat.Property<long>("Created").HasColumnType(BigInt).IsRequired();
        stat.Property<double>("MemoryActive").HasColumnType(Double);
        stat.Property<double>("MemoryCache").HasColumnType(Double);
        stat.Property<double>("CpuUsage").HasColumnType(Double);
        stat.Property<double>("MemoryLimit").HasColumnType(Double);
        stat.Property<double>("RxBytes").HasColumnType(Double);
        stat.Property<double>("TxBytes").HasColumnType(Double);

        stat.HasOne("Platform").WithMany().HasForeignKey("PlatformId").OnDelete(DeleteBehavior.Cascade);
        stat.HasIndex("PlatformId", "DockerTaskId", "Created").IsUnique()
            .HasDatabaseName($"IX_{tableName}_PlatformTaskCreated");
        stat.HasIndex("PlatformId", "DockerServiceId", "Created")
            .HasDatabaseName($"IX_{tableName}_PlatformServiceCreated");
        stat.HasIndex("SwarmServiceId", "Created")
            .HasFilter("swarmserviceid IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_ManagedServiceCreated");
        stat.HasIndex("StackId", "ServiceName", "Created")
            .HasFilter("stackid IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_StackServiceCreated");
        stat.HasIndex("Created").HasDatabaseName($"IX_{tableName}_Created");

        return builder;
    }

    public static ModelBuilder SwarmServiceConfiguration(this ModelBuilder builder)
    {
        const string tableName = "SwarmServices";
        var service = builder.Entity("SwarmService");

        service.ToTable(tableName);
        service.Property<Guid>("Id").IsRequired();
        service.HasKey("Id");
        service.Property<Guid>("PlatformId").IsRequired();
        service.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        service.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        service.Property<string>("DockerName").HasColumnType(Text).HasMaxLength(63).IsRequired();
        service.Property<string>("DockerServiceId").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("Spec").HasColumnType(JsonB).IsRequired();
        service.Property<DateTime?>("AutoUpdateState_LastCheckedAt").HasColumnType(Timestamp).IsRequired(false);
        service.Property<string>("AutoUpdateState_Status").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("AutoUpdateState_CurrentDigest").HasColumnType(Text).IsRequired(false);
        service.Property<string>("AutoUpdateState_RemoteDigest").HasColumnType(Text).IsRequired(false);
        service.Property<string>("AutoUpdateState_LastError").HasColumnType(Text).HasMaxLength(2000).IsRequired(false);
        service.Property<string>("Health").HasColumnType(Text).HasMaxLength(64).IsRequired();
        service.Property<string>("SynchronizationState").HasColumnType(Text).HasMaxLength(64).IsRequired();
        service.Property<string>("DesiredSpecHash").HasColumnType(Text).HasMaxLength(64).IsRequired();
        service.Property<string>("LastAppliedDesiredSpecHash").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("LastAppliedRuntimeHash").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("AppliedImageDigest").HasColumnType(Text).HasMaxLength(1000).IsRequired(false);
        service.Property<long?>("DockerVersionIndex").HasColumnType(BigInt).IsRequired(false);
        service.Property<Guid?>("OperationId").IsRequired(false);
        service.Property<string>("OperationKind").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("OperationState").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<long?>("BaseDockerVersion").HasColumnType(BigInt).IsRequired(false);
        service.Property<string>("TargetDesiredSpecHash").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<string>("TargetRuntimeHash").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<long?>("TargetRowVersion").HasColumnType(BigInt).IsRequired(false);
        service.Property<long?>("ExpectedForceUpdate").HasColumnType(BigInt).IsRequired(false);
        service.Property<DateTime?>("PreparedAt").HasColumnType(Timestamp).IsRequired(false);
        service.Property<DateTime?>("AttemptedAt").HasColumnType(Timestamp).IsRequired(false);
        service.Property<DateTime?>("CompletedAt").HasColumnType(Timestamp).IsRequired(false);
        service.Property<long?>("ObservedDockerVersion").HasColumnType(BigInt).IsRequired(false);
        service.Property<string>("ResultCode").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        service.Property<string>("OperationClusterId").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        service.Property<Guid?>("OperationActorId").IsRequired(false);
        service.Property<string>("Warnings").HasColumnType(JsonB).IsRequired(false);
        service.Property<string>("ResultMessage").HasColumnType(Text).HasMaxLength(2000).IsRequired(false);
        service.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired();

        service.AddReconcilableMember(requireControlState: true).AddAuditedMemebers();
        service.HasOne("Platform").WithMany().HasForeignKey("PlatformId").OnDelete(DeleteBehavior.Restrict);
        service.HasIndex("Name", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_Name_PlatformId");
        service.HasIndex("DockerName", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_DockerName_PlatformId");
        service.HasIndex("PlatformId", "DockerServiceId").IsUnique()
            .HasFilter("\"dockerserviceid\" IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_PlatformId_DockerServiceId");
        service.HasIndex("OperationState", "PreparedAt").HasDatabaseName($"IX_{tableName}_RecoverableOperation");

        service.ToTable(t => t.HasCheckConstraint(
            $"CK_{tableName}_OperationFields",
            "(operationid IS NULL AND operationkind IS NULL AND operationstate IS NULL AND basedockerversion IS NULL " +
            "AND targetdesiredspechash IS NULL AND targetruntimehash IS NULL AND targetrowversion IS NULL " +
            "AND expectedforceupdate IS NULL AND preparedat IS NULL AND attemptedat IS NULL AND completedat IS NULL " +
            "AND observeddockerversion IS NULL AND resultcode IS NULL AND warnings IS NULL AND resultmessage IS NULL " +
            "AND operationclusterid IS NULL AND operationactorid IS NULL) " +
            "OR (operationid IS NOT NULL AND operationkind IS NOT NULL AND operationstate IS NOT NULL " +
            "AND targetdesiredspechash IS NOT NULL AND targetrowversion IS NOT NULL AND preparedat IS NOT NULL " +
            "AND operationclusterid IS NOT NULL AND operationactorid IS NOT NULL)"));

        service.ToTable(t => t.HasCheckConstraint(
            $"CK_{tableName}_CanceledOperation",
            "operationstate <> 'Canceled' OR (attemptedat IS NULL AND completedat IS NOT NULL)"));

        ConfigureGlobalSearchIndex(service, tableName, "Name");
        return builder;
    }

    public static ModelBuilder SwarmNodeProjectionConfiguration(this ModelBuilder builder)
    {
        var tableName = "SwarmNodeProjections";
        var node = builder.Entity("SwarmNodeProjection");

        node.ToTable(tableName);
        node.Property<Guid>("PlatformId").IsRequired();
        node.Property<string>("DockerNodeId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        node.HasKey("PlatformId", "DockerNodeId");
        node.Property<long>("VersionIndex").HasColumnType(BigInt).IsRequired();
        node.Property<string>("Hostname").HasColumnType(Text).HasMaxLength(255).IsRequired();
        node.Property<string>("Role").HasColumnType(Text).HasMaxLength(32).IsRequired();
        node.Property<bool>("IsLeader").IsRequired();
        node.Property<string>("Reachability").HasColumnType(Text).HasMaxLength(32).IsRequired();
        node.Property<string>("Status").HasColumnType(Text).HasMaxLength(32).IsRequired();
        node.Property<string>("StatusMessage").HasColumnType(Text).HasMaxLength(1000).IsRequired(false);
        node.Property<string>("Availability").HasColumnType(Text).HasMaxLength(32).IsRequired();
        node.Property<string>("EngineVersion").HasColumnType(Text).HasMaxLength(64).IsRequired();
        node.Property<string>("OperatingSystem").HasColumnType(Text).HasMaxLength(64).IsRequired();
        node.Property<string>("Architecture").HasColumnType(Text).HasMaxLength(64).IsRequired();
        node.Property<string>("Address").HasColumnType(Text).HasMaxLength(255).IsRequired();
        node.Property<string>("Labels").HasColumnType(JsonB).IsRequired();
        node.Property<int>("RunningTaskCount").HasColumnType(Integer).IsRequired();
        node.Property<int>("DesiredTaskCount").HasColumnType(Integer).IsRequired();
        node.Property<DateTimeOffset?>("DockerCreatedAt").HasColumnType(Timestamp).IsRequired(false);
        node.Property<DateTimeOffset?>("DockerUpdatedAt").HasColumnType(Timestamp).IsRequired(false);
        node.Property<DateTimeOffset>("ObservedAt").HasColumnType(Timestamp).IsRequired();
        node.Property<bool>("IsStale").IsRequired().HasDefaultValue(false);

        node
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        return builder;
    }

    public static ModelBuilder SwarmServiceProjectionConfiguration(this ModelBuilder builder)
    {
        var service = builder.Entity("SwarmServiceProjection");
        service.ToTable("SwarmServiceProjections");
        service.Property<Guid>("PlatformId").IsRequired();
        service.Property<string>("DockerServiceId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        service.HasKey("PlatformId", "DockerServiceId");
        service.Property<long>("VersionIndex").HasColumnType(BigInt).IsRequired();
        service.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        service.Property<string>("Mode").HasColumnType(Text).HasMaxLength(32).IsRequired();
        service.Property<string>("Image").HasColumnType(Text).HasMaxLength(1000).IsRequired();
        service.Property<int>("RunningTaskCount").HasColumnType(Integer).IsRequired();
        service.Property<int>("DesiredTaskCount").HasColumnType(Integer).IsRequired();
        service.Property<string>("UpdateState").HasColumnType(Text).HasMaxLength(64).IsRequired();
        service.Property<string>("UpdateMessage").HasColumnType(Text).HasMaxLength(1000).IsRequired(false);
        service.Property<string>("Ports").HasColumnType(JsonB).IsRequired();
        service.Property<string>("NetworkIds").HasColumnType(JsonB).IsRequired();
        service.Property<string>("SecretIds").HasColumnType(JsonB).IsRequired();
        service.Property<string>("ConfigIds").HasColumnType(JsonB).IsRequired();
        service.Property<string>("Labels").HasColumnType(JsonB).IsRequired();
        service.Property<string>("Ownership").HasColumnType(Text).HasMaxLength(32).IsRequired()
            .HasDefaultValue(nameof(SwarmServiceOwnership.Unmanaged));
        service.Property<string>("DockerStackNamespace").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        service.Property<string>("OwnershipDiagnostic").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        service.Property<Guid?>("SwarmServiceId").IsRequired(false);
        service.Property<Guid?>("StackId").IsRequired(false);
        service.Property<string>("LiveRuntimeHash").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        service.Property<long>("ForceUpdate").HasDefaultValue(0L);
        service.HasIndex("SwarmServiceId").HasDatabaseName("IX_SwarmServiceProjections_SwarmServiceId");
        service.HasIndex("StackId").HasDatabaseName("IX_SwarmServiceProjections_StackId");
        service
            .HasOne("Stack")
            .WithMany()
            .HasForeignKey("StackId")
            .OnDelete(DeleteBehavior.SetNull);
        AddSwarmObservationFields(service);
        AddSwarmPlatformRelationship(service);
        return builder;
    }

    public static ModelBuilder SwarmTaskProjectionConfiguration(this ModelBuilder builder)
    {
        var task = builder.Entity("SwarmTaskProjection");
        task.ToTable("SwarmTaskProjections");
        task.Property<Guid>("PlatformId").IsRequired();
        task.Property<string>("DockerTaskId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        task.HasKey("PlatformId", "DockerTaskId");
        task.Property<long>("VersionIndex").HasColumnType(BigInt).IsRequired();
        task.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        task.Property<string>("DockerServiceId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        task.Property<string>("ServiceName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        task.Property<int?>("Slot").HasColumnType(Integer).IsRequired(false);
        task.Property<string>("DockerNodeId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        task.Property<string>("DockerContainerId").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        task.Property<string>("NodeHostname").HasColumnType(Text).HasMaxLength(255).IsRequired();
        task.Property<string>("DesiredState").HasColumnType(Text).HasMaxLength(32).IsRequired();
        task.Property<string>("State").HasColumnType(Text).HasMaxLength(32).IsRequired();
        task.Property<string>("StatusMessage").HasColumnType(Text).HasMaxLength(1000).IsRequired(false);
        task.Property<string>("Error").HasColumnType(Text).HasMaxLength(1000).IsRequired(false);
        task.Property<string>("Image").HasColumnType(Text).HasMaxLength(1000).IsRequired();
        task.Property<string>("Ports").HasColumnType(JsonB).IsRequired();
        task.Property<DateTimeOffset?>("StatusTimestamp").HasColumnType(Timestamp).IsRequired(false);
        AddSwarmObservationFields(task);
        AddSwarmPlatformRelationship(task);
        return builder;
    }

    public static ModelBuilder SwarmNetworkProjectionConfiguration(this ModelBuilder builder)
    {
        var network = builder.Entity("SwarmNetworkProjection");
        network.ToTable("SwarmNetworkProjections");
        network.Property<Guid>("PlatformId").IsRequired();
        network.Property<string>("DockerNetworkId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        network.HasKey("PlatformId", "DockerNetworkId");
        network.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        network.Property<string>("Scope").HasColumnType(Text).HasMaxLength(32).IsRequired();
        network.Property<string>("Driver").HasColumnType(Text).HasMaxLength(64).IsRequired();
        network.Property<bool>("IsAttachable").IsRequired();
        network.Property<bool>("IsInternal").IsRequired();
        network.Property<bool>("IsIngress").IsRequired();
        network.Property<bool>("IsEncrypted").IsRequired();
        network.Property<bool>("EnableIPv6").IsRequired();
        network.Property<string>("Subnets").HasColumnType(JsonB).IsRequired();
        network.Property<string>("ServiceNames").HasColumnType(JsonB).IsRequired();
        network.Property<string>("Labels").HasColumnType(JsonB).IsRequired();
        network.Property<DateTimeOffset?>("DockerCreatedAt").HasColumnType(Timestamp).IsRequired(false);
        network.Property<DateTimeOffset>("ObservedAt").HasColumnType(Timestamp).IsRequired();
        network.Property<bool>("IsStale").IsRequired().HasDefaultValue(false);
        AddSwarmPlatformRelationship(network);
        return builder;
    }

    public static ModelBuilder SwarmSecretProjectionConfiguration(this ModelBuilder builder)
    {
        var secret = builder.Entity("SwarmSecretProjection");
        secret.ToTable("SwarmSecretProjections");
        secret.Property<Guid>("PlatformId").IsRequired();
        secret.Property<string>("DockerSecretId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        secret.HasKey("PlatformId", "DockerSecretId");
        secret.Property<long>("VersionIndex").HasColumnType(BigInt).IsRequired();
        secret.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        secret.Property<string>("Driver").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        secret.Property<string>("ServiceNames").HasColumnType(JsonB).IsRequired();
        secret.Property<string>("Labels").HasColumnType(JsonB).IsRequired();
        AddSwarmObservationFields(secret);
        AddSwarmPlatformRelationship(secret);
        return builder;
    }

    public static ModelBuilder SwarmConfigProjectionConfiguration(this ModelBuilder builder)
    {
        var config = builder.Entity("SwarmConfigProjection");
        config.ToTable("SwarmConfigProjections");
        config.Property<Guid>("PlatformId").IsRequired();
        config.Property<string>("DockerConfigId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        config.HasKey("PlatformId", "DockerConfigId");
        config.Property<long>("VersionIndex").HasColumnType(BigInt).IsRequired();
        config.Property<string>("Name").HasColumnType(Text).HasMaxLength(255).IsRequired();
        config.Property<string>("TemplatingDriver").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        config.Property<string>("ServiceNames").HasColumnType(JsonB).IsRequired();
        config.Property<string>("Labels").HasColumnType(JsonB).IsRequired();
        AddSwarmObservationFields(config);
        AddSwarmPlatformRelationship(config);
        return builder;
    }

    public static ModelBuilder SwarmNodeRuntimeProjectionStateConfiguration(this ModelBuilder builder)
    {
        var state = builder.Entity("SwarmNodeRuntimeProjectionState");
        state.ToTable("SwarmNodeRuntimeProjectionStates");
        state.Property<Guid>("PlatformId").IsRequired();
        state.Property<string>("DockerNodeId").HasColumnType(Text).HasMaxLength(64).IsRequired();
        state.HasKey("PlatformId", "DockerNodeId");
        state.Property<long>("ReconciliationGeneration").HasColumnType(BigInt).IsRequired().HasDefaultValue(0L);
        state.Property<DateTimeOffset?>("ReconciliationStartedAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<DateTimeOffset?>("ReconciliationCompletedAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<DateTimeOffset?>("LastSuccessfulReconciliationAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<bool>("IsStale").IsRequired().HasDefaultValue(true);
        state.Property<DateTimeOffset?>("StaleSince").HasColumnType(Timestamp).IsRequired(false);
        state.Property<string>("StaleReason").HasColumnType(Text).HasMaxLength(512).IsRequired(false);
        state.Property<DateTimeOffset?>("LastEventStreamConnectedAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<DateTimeOffset?>("LastEventGapAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<DateTimeOffset?>("LastStatsSampleAt").HasColumnType(Timestamp).IsRequired(false);
        state.Property<string>("AgentVersion").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        state.Property<string>("DockerVersion").HasColumnType(Text).HasMaxLength(64).IsRequired(false);
        AddSwarmPlatformRelationship(state);
        return builder;
    }

    private static void AddSwarmObservationFields(EntityTypeBuilder entity)
    {
        entity.Property<DateTimeOffset?>("DockerCreatedAt").HasColumnType(Timestamp).IsRequired(false);
        entity.Property<DateTimeOffset?>("DockerUpdatedAt").HasColumnType(Timestamp).IsRequired(false);
        entity.Property<DateTimeOffset>("ObservedAt").HasColumnType(Timestamp).IsRequired();
        entity.Property<bool>("IsStale").IsRequired().HasDefaultValue(false);
    }

    private static void AddSwarmPlatformRelationship(EntityTypeBuilder entity) =>
        entity.HasOne("Platform").WithMany().HasForeignKey("PlatformId").OnDelete(DeleteBehavior.Cascade);

    public static ModelBuilder BuildAgentPoolConfiguration(this ModelBuilder builder)
    {
        var table = "BuildAgentPools";
        var pool = builder.Entity("BuildAgentPool");
        pool.ToTable(table);
        pool.Property<Guid>("Id").IsRequired();
        pool.HasKey("Id");
        pool.Property<string>("Name").HasColumnType(Text).HasMaxLength(128).IsRequired();
        pool.Property<string>("NormalizedName").HasColumnType(Text).HasMaxLength(128).IsRequired();
        pool.Property<string>("Description").HasColumnType(Text).HasMaxLength(600).IsRequired(false);
        pool.Property<bool>("Enabled").HasColumnType("boolean").IsRequired().HasDefaultValue(true);
        pool.Property<string>("Provider").HasColumnType(Text).HasMaxLength(64).IsRequired();
        pool.Property<string>("ProviderSpec").HasColumnType("jsonb").IsRequired();
        pool.Property<int>("MaxActiveBuilders").HasColumnType(Integer).IsRequired().HasDefaultValue(1);
        pool.Property<int>("QueueTimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(3600);
        pool.Property<int>("ProvisioningTimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(600);
        pool.Property<int>("RegistrationTimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(300);
        pool.Property<int>("HeartbeatTimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(90);
        pool.Property<int>("CleanupTimeoutSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(600);
        pool.Property<int>("MaximumInstanceLifetimeSeconds").HasColumnType(Integer).IsRequired().HasDefaultValue(7200);
        pool.Property<int>("FailureRetentionMinutes").HasColumnType(Integer).IsRequired().HasDefaultValue(0);
        pool.Property<string>("LastValidationStatus").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue("NotTested");
        pool.Property<string>("LastValidationMessage").HasColumnType(Text).HasMaxLength(1200).IsRequired(false);
        pool.Property<DateTime?>("LastValidatedAt").HasColumnType(Timestamp).IsRequired(false);
        pool.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        pool.Property<DateTime?>("ArchivedAt").HasColumnType(Timestamp).IsRequired(false);
        pool
            .AddReconcilableMember(requireControlState: true)
            .AddAuditedMemebers();

        pool.HasIndex("NormalizedName").IsUnique().HasDatabaseName($"IX_{table}_NormalizedName");
        pool.HasIndex("Provider").HasDatabaseName($"IX_{table}_Provider");
        pool.HasIndex("Enabled").HasDatabaseName($"IX_{table}_Enabled");
        pool.HasIndex("ArchivedAt").HasDatabaseName($"IX_{table}_ArchivedAt");
        ConfigureGlobalSearchIndex(pool, table, "Name", activeOnly: true);
        ConfigureGlobalSearchIndex(pool, table, "Provider", activeOnly: true);

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
        stack.Property<string>("DriftPolicy").HasColumnType(Json).IsRequired();

        stack
            .AddReconcilableMember()
            .AddAuditedMemebers();

        // Uniqueness will be enforced at the application level since stacks can be shared across platforms and may have the same name
        // stack.HasIndex("Name", "PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_Name_PlatformId");
        stack.HasIndex("CurrentStackReleaseId").HasDatabaseName($"IX_{tableName}_CurrentStackReleaseId");
        ConfigureGlobalSearchIndex(stack, tableName, "Name");

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
        release.Property<string>("Spec").HasColumnType(JsonB).IsRequired();
        release.Property<string>("Source").HasColumnType(Json).IsRequired(false);
        release.Property<string>("ResourceBindings").HasColumnType(Json).IsRequired(false);

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

    public static ModelBuilder StackReleaseVolumeBindingConfiguration(this ModelBuilder builder)
    {
        var tableName = "StackReleaseVolumeBindings";
        var binding = builder.Entity("StackReleaseVolumeBinding");

        binding.ToTable(tableName);
        binding.Property<Guid>("Id").IsRequired();
        binding.HasKey("Id");
        binding.Property<Guid>("StackReleaseId").IsRequired();
        binding.Property<Guid>("PlatformId").IsRequired();
        binding.Property<string>("VolumeName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        binding.Property<string>("ComposeVolumeName").HasColumnType(Text).HasMaxLength(255).IsRequired(false);
        binding.Property<bool>("IsExternal").HasColumnType("boolean").IsRequired().HasDefaultValue(false);
        binding.Property<bool>("IsAnonymous").HasColumnType("boolean").IsRequired().HasDefaultValue(false);
        binding.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        binding
            .HasOne("StackRelease")
            .WithMany()
            .HasForeignKey("StackReleaseId")
            .OnDelete(DeleteBehavior.Cascade);

        binding
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Restrict);

        binding.HasIndex("StackReleaseId", "VolumeName").IsUnique().HasDatabaseName($"IX_{tableName}_Release_VolumeName");
        binding.HasIndex("PlatformId", "VolumeName").HasDatabaseName($"IX_{tableName}_Platform_VolumeName");

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
        alertEvent.Property<string>("UnmanagedContainerId")
            .HasColumnType(Text)
            .HasComputedColumnSql(
                "CASE WHEN type = 'UnmanagedContainerCreated' THEN info->>'ContainerId' ELSE NULL END",
                stored: true)
            .IsRequired(false);
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
        alertEvent.HasIndex("UnmanagedContainerId")
            .HasFilter("resolvedat IS NULL AND unmanagedcontainerid IS NOT NULL")
            .HasDatabaseName($"IX_{tableName}_Open_UnmanagedContainerId");

        return builder;
    }

    private static EntityTypeBuilder AddReconcilableMember(
        this EntityTypeBuilder builder,
        bool includeControlTriggeredBy = true,
        bool requireControlState = false)
    {
        builder.Property<long>("RowVersion").HasColumnType(BigInt).IsRequired().HasDefaultValue(0L);
        builder.Property<long?>("ControlStartedAt").HasColumnType(BigInt).HasDefaultValue(null);
        var controlState = builder.Property<string>("ControlState").HasColumnType(Text).HasMaxLength(64).HasDefaultValue(ResourceControlState.Idle.ToString());
        if (requireControlState)
            controlState.IsRequired();

        if (includeControlTriggeredBy)
        {
            builder.Property<Guid?>("ControlTriggeredBy").HasColumnType("uuid").IsRequired(false);

            builder
                .HasOne("Actor")
                .WithMany()
                .HasForeignKey("ControlTriggeredBy")
                .OnDelete(DeleteBehavior.Restrict);
        }

        return builder;
    }

    public static ModelBuilder StackReleaseSwarmResourceConfiguration(this ModelBuilder builder)
    {
        var tableName = "StackReleaseSwarmResources";
        var resource = builder.Entity("StackReleaseSwarmResource");

        resource.ToTable(tableName);
        resource.Property<Guid>("Id").IsRequired();
        resource.HasKey("Id");
        resource.Property<Guid>("StackReleaseId").IsRequired();
        resource.Property<Guid>("PlatformId").IsRequired();
        resource.Property<string>("Kind").HasColumnType(Text).IsRequired();
        resource.Property<string>("DockerResourceId").HasColumnType(Text).HasMaxLength(255).IsRequired();
        resource.Property<string>("DockerResourceName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        resource.Property<string>("ComposeResourceName").HasColumnType(Text).HasMaxLength(255).IsRequired();
        resource.Property<string>("Mounts").HasColumnType(Json).IsRequired();

        resource
            .HasOne("StackRelease")
            .WithMany()
            .HasForeignKey("StackReleaseId")
            .OnDelete(DeleteBehavior.Cascade);

        resource
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Restrict);

        resource
            .HasIndex("StackReleaseId", "Kind", "DockerResourceId")
            .IsUnique()
            .HasDatabaseName($"IX_{tableName}_Release_Kind_ResourceId");
        resource
            .HasIndex("PlatformId", "Kind", "DockerResourceName")
            .HasDatabaseName($"IX_{tableName}_Platform_Kind_Name");

        return builder;
    }

    public static ModelBuilder StackSwarmNamespaceReservationConfiguration(this ModelBuilder builder)
    {
        const string tableName = "StackSwarmNamespaceReservations";
        var reservation = builder.Entity("StackSwarmNamespaceReservation");

        reservation.ToTable(tableName);
        reservation.Property<Guid>("StackId").IsRequired();
        reservation.HasKey("StackId");
        reservation.Property<Guid>("PlatformId").IsRequired();
        reservation.Property<string>("Namespace").HasColumnType(Text).HasMaxLength(63).IsRequired();
        reservation.Property<DateTime>("CreatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        reservation
            .HasOne("Stack")
            .WithOne()
            .HasForeignKey("StackSwarmNamespaceReservation", "StackId")
            .OnDelete(DeleteBehavior.Cascade);

        reservation
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Restrict);

        reservation.HasIndex("PlatformId", "Namespace")
            .IsUnique()
            .HasDatabaseName($"IX_{tableName}_Platform_Namespace");

        return builder;
    }

    public static ModelBuilder StackWebhookDeployQueueConfiguration(this ModelBuilder builder)
    {
        var tableName = "StackWebhookDeployQueue";
        var item = builder.Entity("StackWebhookDeployQueueItem");

        item.ToTable(tableName);
        item.Property<Guid>("Id").IsRequired();
        item.HasKey("Id");
        item.Property<Guid>("StackId").IsRequired();
        item.Property<Guid>("GitRepositoryId").IsRequired();
        item.Property<Guid>("ExpectedStackReleaseId").IsRequired();
        item.Property<string>("Branch").HasColumnType(Text).HasMaxLength(256).IsRequired();
        item.Property<string>("ExpectedSpecFingerprint").HasColumnType(Text).HasMaxLength(64).IsRequired();
        item.Property<string>("DispatchedCommitSha").HasColumnType(Text).HasMaxLength(128).IsRequired(false);
        item.Property<string>("Status").HasColumnType(Text).HasMaxLength(32).IsRequired();
        item.Property<int>("Attempts").HasColumnType(Integer).IsRequired();
        item.Property<DateTime>("QueuedAt").HasColumnType(Timestamp).IsRequired();
        item.Property<DateTime>("AvailableAt").HasColumnType(Timestamp).IsRequired();
        item.Property<DateTime?>("StartedAt").HasColumnType(Timestamp).IsRequired(false);
        item.Property<string>("LastError").HasColumnType(Text).HasMaxLength(2000).IsRequired(false);

        item
            .HasOne("Stack")
            .WithMany()
            .HasForeignKey("StackId")
            .OnDelete(DeleteBehavior.Cascade);

        item
            .HasOne("GitRepository")
            .WithMany()
            .HasForeignKey("GitRepositoryId")
            .OnDelete(DeleteBehavior.Cascade);

        item.HasIndex("Status", "AvailableAt", "QueuedAt")
            .HasDatabaseName($"IX_{tableName}_Ready");
        item.HasIndex("StackId").HasDatabaseName($"IX_{tableName}_StackId");
        item.HasIndex("GitRepositoryId").HasDatabaseName($"IX_{tableName}_GitRepositoryId");

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
