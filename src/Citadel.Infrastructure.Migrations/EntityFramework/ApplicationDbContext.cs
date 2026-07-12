using Domain;
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
        modelBuilder
            .ContainerConfiguration()
            .ContainerStatConfiguration()
            .PlatformConfiguration()
            .EdgeAgentEnrollmentConfiguration()
            .EdgeAgentBindingConfiguration()
            .PlatformStatConfiguration()
            .RegistryConfiguration()
            .GitAccountConfiguration()
            .GitRepositoryConfiguration()
            .GitRepositoryRefConfiguration()
            .RefreshTokenConfiguration()
            .UserPreferencesConfiguration()
            .CitadelInstanceIdentityConfiguration()
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
            .ImageConfiguration()
            .AutomationActionConfiguration()
            .ActionRunConfiguration()
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

        // Tags
        var SystemTagId = Guid.Parse("40000000-0000-0000-0000-000000000001");
        var prodTagId = Guid.Parse("40000000-0000-0000-0000-000000000002");

        // Automation actions
        var pruneImagesActionId = Guid.Parse("41000000-0000-0000-0000-000000000001");
        var restartProdStacksActionId = Guid.Parse("41000000-0000-0000-0000-000000000002");

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
            // Admin user -> Admin role
            new { ActorId = adminActorId, RoleId = adminRoleId },

            // Team -> Operator role
            new { ActorId = teamActorId, RoleId = operatorRoleId }
        );

        // Permissions — only valid combinations from the matrix are seeded
        var permissions = new List<object>();

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

            if (resource == ResourceType.License)
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

        // Automation actions
        modelBuilder.Entity("AutomationAction").HasData(
            new
            {
                Id = pruneImagesActionId,
                Name = "Prune images",
                Description = "Prunes unused Docker images on every platform.",
                Code = """
                    const platformsResponse = await citadel.platforms.listPlatforms();
                    const platforms = platformsResponse?.platforms ?? [];
                    let pruned = 0;
                    let reclaimedBytes = 0;

                    for (const platform of platforms) {
                      const result = await citadel.platforms.prunePlatform(platform.id, { resource: "Image" });
                      const imagesDeleted = result?.imagesDeleted ?? [];
                      const reclaimed = Number(result?.spaceReclaimed ?? 0);
                      reclaimedBytes += reclaimed;
                      pruned += imagesDeleted.length;

                      if (imagesDeleted.length === 0) {
                        console.log(`No unused images on ${platform.name}.`);
                        continue;
                      }

                      console.log(`Pruned ${imagesDeleted.length} image item(s) on ${platform.name}; reclaimed ${reclaimed} bytes.`);
                    }

                    console.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);
                    """.Replace("\r\n", "\n"),
                DefaultArgsJson = "{}",
                Enabled = false,
                ScheduleEnabled = true,
                ScheduleCron = "0 12 * * *",
                ScheduleTimeZone = "UTC",
                Webhook = (string?)null,
                TimeoutSeconds = 300,
                AlertOnFailure = true,
                RunAsActorId = adminActorId,
                LastScheduledRunAt = (DateTime?)null,
                ControlState = ResourceControlState.Idle.ToString(),
                CurrentRunId = (Guid?)null,
                RowVersion = 0L,
                CreatedByActorId = systemActorId,
                CreatedAt = seedDate,
                UpdatedAt = seedDate
            },
            new
            {
                Id = restartProdStacksActionId,
                Name = "Restart unhealthy stacks",
                Description = "Restarts stacks tagged Prod when their current release is not healthy.",
                Code = """
                    const stacksResponse = await citadel.stacks.listStacks({ tags: ["Prod"] });
                    const stacks = stacksResponse?.stacks ?? [];
                    const unhealthyStacks = stacks.filter(
                      (stack) => stack.status !== "Healthy" && stack.controlState !== "Processing"
                    );

                    if (unhealthyStacks.length === 0) {
                      console.log("No unhealthy Prod stacks found.");
                    } else {
                      const stackIds = unhealthyStacks.map((stack) => stack.id);
                      await citadel.stacks.restartStacks(stackIds);
                      console.log(`Requested restart for ${stackIds.length} Prod stack(s).`);
                    }
                    """.Replace("\r\n", "\n"),
                DefaultArgsJson = "{}",
                Enabled = false,
                ScheduleEnabled = true,
                ScheduleCron = "*/15 * * * *",
                ScheduleTimeZone = "UTC",
                Webhook = (string?)null,
                TimeoutSeconds = 300,
                AlertOnFailure = true,
                RunAsActorId = adminActorId,
                LastScheduledRunAt = (DateTime?)null,
                ControlState = ResourceControlState.Idle.ToString(),
                CurrentRunId = (Guid?)null,
                RowVersion = 0L,
                CreatedByActorId = systemActorId,
                CreatedAt = seedDate,
                UpdatedAt = seedDate
            });

        modelBuilder.Entity("ResourceTag").HasData(
            new
            {
                ResourceType = TaggableResourceType.AutomationAction.ToString(),
                ResourceId = pruneImagesActionId,
                TagId = SystemTagId,
                CreatedAt = seedDate,
                CreatedByActorId = systemActorId
            },
            new
            {
                ResourceType = TaggableResourceType.AutomationAction.ToString(),
                ResourceId = restartProdStacksActionId,
                TagId = SystemTagId,
                CreatedAt = seedDate,
                CreatedByActorId = systemActorId
            },
            new
            {
                ResourceType = TaggableResourceType.AutomationAction.ToString(),
                ResourceId = restartProdStacksActionId,
                TagId = prodTagId,
                CreatedAt = seedDate,
                CreatedByActorId = systemActorId
            });

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
    private const string Integer = "integer";
    private const string BigInt = "bigint";
    private const string Double = "double precision";
    private const string Timestamp = "timestamp with time zone";

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

        platform.HasIndex("Address").IsUnique().HasDatabaseName($"IX_{tableName}_Address");

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
        enrollment.Property<string>("TokenHash").HasColumnType(Text).HasMaxLength(128).IsRequired();
        enrollment.Property<DateTime>("ExpiresAtUtc").HasColumnType(Timestamp).IsRequired();
        enrollment.Property<DateTime?>("UsedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        enrollment.Property<DateTime?>("RevokedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        enrollment.Property<Guid>("CreatedByActorId").IsRequired();
        enrollment.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        enrollment
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        enrollment
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("CreatedByActorId")
            .OnDelete(DeleteBehavior.Restrict);

        enrollment.HasIndex("PlatformId", "ExpiresAtUtc").HasDatabaseName($"IX_{tableName}_PlatformId_ExpiresAtUtc");
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
        binding.Property<DateTime?>("RevokedAtUtc").HasColumnType(Timestamp).IsRequired(false);
        binding.Property<DateTime>("CreatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");
        binding.Property<DateTime>("UpdatedAtUtc").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        binding
            .HasOne("Platform")
            .WithMany()
            .HasForeignKey("PlatformId")
            .OnDelete(DeleteBehavior.Cascade);

        binding.HasIndex("PlatformId").IsUnique().HasDatabaseName($"IX_{tableName}_PlatformId");
        binding.HasIndex("AgentId").HasDatabaseName($"IX_{tableName}_AgentId");
        binding.HasIndex("AgentFingerprint").HasDatabaseName($"IX_{tableName}_AgentFingerprint");

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
        action.Property<string>("ControlState").HasColumnType(Text).HasMaxLength(64).IsRequired().HasDefaultValue(ResourceControlState.Idle.ToString());
        action.Property<Guid?>("CurrentRunId").IsRequired(false);
        action.Property<long>("RowVersion").HasColumnType(BigInt).IsRequired().HasDefaultValue(0L);
        action.Property<DateTime>("UpdatedAt").HasColumnType(Timestamp).IsRequired().HasDefaultValueSql("CURRENT_TIMESTAMP");

        action.AddAuditedMemebers();

        action
            .HasOne("Actor")
            .WithMany()
            .HasForeignKey("RunAsActorId")
            .OnDelete(DeleteBehavior.Restrict);

        action.HasIndex("Name").IsUnique().HasDatabaseName($"IX_{tableName}_Name");
        action.HasIndex("CreatedByActorId").HasDatabaseName($"IX_{tableName}_CreatedByActorId");
        action.HasIndex("RunAsActorId").HasDatabaseName($"IX_{tableName}_RunAsActorId");
        action.HasIndex("Enabled", "ScheduleEnabled", "ScheduleCron").HasDatabaseName($"IX_{tableName}_Schedule");

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

        run.HasIndex("ActionId", "QueuedAt").HasDatabaseName($"IX_{tableName}_ActionId_QueuedAt");
        run.HasIndex("Status", "QueuedAt").HasDatabaseName($"IX_{tableName}_Status_QueuedAt");
        run.HasIndex("RunAsActorId").HasDatabaseName($"IX_{tableName}_RunAsActorId");
        run.HasIndex("TriggeredByActorId").HasDatabaseName($"IX_{tableName}_TriggeredByActorId");

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
