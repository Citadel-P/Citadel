CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
    "ProductVersion" TEXT NOT NULL
);

BEGIN TRANSACTION;
CREATE TABLE "Actors" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Actors" PRIMARY KEY,
    "Type" TEXT NOT NULL,
    "IsEnabled" INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE "Platforms" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Platforms" PRIMARY KEY,
    "Address" TEXT NOT NULL,
    "AgentVersion" TEXT NULL,
    "ConnectorType" TEXT NOT NULL,
    "CpuCount" REAL NOT NULL,
    "ImageCount" REAL NOT NULL,
    "MemTotal" REAL NOT NULL,
    "Name" TEXT NOT NULL,
    "NetworkCount" REAL NOT NULL,
    "PlatformDescriptor" TEXT NOT NULL,
    "ServerVersion" TEXT NULL,
    "Status" TEXT NOT NULL,
    "VolumeCount" REAL NOT NULL
);

CREATE TABLE "ResourceAccesses" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_ResourceAccesses" PRIMARY KEY,
    "Action" TEXT NOT NULL,
    "ActorId" TEXT NOT NULL,
    "ResourceId" TEXT NOT NULL,
    "ResourceType" TEXT NOT NULL
);

CREATE TABLE "Roles" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Roles" PRIMARY KEY,
    "Name" TEXT NOT NULL
);

CREATE TABLE "AlertChannels" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_AlertChannels" PRIMARY KEY,
    "AlertDestination" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "IsActive" INTEGER NOT NULL DEFAULT 1,
    "Name" TEXT NOT NULL,
    "Url" TEXT NOT NULL,
    CONSTRAINT "FK_AlertChannels_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "AlertRules" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_AlertRules" PRIMARY KEY,
    "CooldownSeconds" INTEGER NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Description" TEXT NULL,
    "LimitedTo" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "QuietHours" TEXT NOT NULL,
    "RequiredMatches" INTEGER NULL,
    "Severity" TEXT NOT NULL,
    "Status" TEXT NOT NULL DEFAULT 'Enabled',
    "Threshold" REAL NULL,
    "Type" TEXT NOT NULL,
    CONSTRAINT "FK_AlertRules_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "GitAccounts" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_GitAccounts" PRIMARY KEY,
    "AuthType" TEXT NOT NULL,
    "Configuration" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Domain" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "Transport" TEXT NOT NULL,
    CONSTRAINT "FK_GitAccounts_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Registries" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Registries" PRIMARY KEY,
    "Configuration" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Description" TEXT NULL,
    "Name" TEXT NOT NULL,
    "RegistryHost" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    CONSTRAINT "FK_Registries_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Stacks" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Stacks" PRIMARY KEY,
    "ControlStartedAt" INTEGER NULL,
    "ControlState" TEXT NULL DEFAULT 'Idle',
    "ControlTriggeredBy" TEXT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "CurrentStackReleaseId" TEXT NULL,
    "Description" TEXT NULL,
    "Name" TEXT NOT NULL,
    "RowVersion" INTEGER NOT NULL DEFAULT 0,
    "StackSource" TEXT NOT NULL,
    "StackUpdateState" TEXT NOT NULL,
    CONSTRAINT "FK_Stacks_Actors_ControlTriggeredBy" FOREIGN KEY ("ControlTriggeredBy") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Stacks_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Teams" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Teams" PRIMARY KEY,
    "ActorId" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    CONSTRAINT "FK_Teams_Actors_ActorId" FOREIGN KEY ("ActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "Users" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Users" PRIMARY KEY,
    "ActorId" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Email" TEXT NULL,
    "Name" TEXT NOT NULL,
    "Password" TEXT NULL,
    CONSTRAINT "FK_Users_Actors_ActorId" FOREIGN KEY ("ActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Users_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "ActivityEvents" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_ActivityEvents" PRIMARY KEY,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "EventType" TEXT NOT NULL,
    "Info" TEXT NOT NULL,
    "PlatformId" TEXT NULL,
    "ResourceId" TEXT NULL,
    "ResourceName" TEXT NOT NULL,
    "ResourceType" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    CONSTRAINT "FK_ActivityEvents_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_ActivityEvents_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Deployments" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Deployments" PRIMARY KEY,
    "AutoUpdateState_CurrentDigest" TEXT NULL,
    "AutoUpdateState_LastCheckedAt" TEXT NULL,
    "AutoUpdateState_LastError" TEXT NULL,
    "AutoUpdateState_RemoteDigest" TEXT NULL,
    "AutoUpdateState_Status" TEXT NULL,
    "ControlStartedAt" INTEGER NULL,
    "ControlState" TEXT NULL DEFAULT 'Idle',
    "ControlTriggeredBy" TEXT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Description" TEXT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "RowVersion" INTEGER NOT NULL DEFAULT 0,
    "Spec" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    CONSTRAINT "FK_Deployments_Actors_ControlTriggeredBy" FOREIGN KEY ("ControlTriggeredBy") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Deployments_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Deployments_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE RESTRICT
);

CREATE TABLE "PlatformStats" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_PlatformStats" PRIMARY KEY,
    "CpuUsage" REAL NOT NULL,
    "Created" REAL NOT NULL,
    "MemoryUsage" REAL NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "RxBytes" REAL NOT NULL,
    "TxBytes" REAL NOT NULL,
    CONSTRAINT "FK_PlatformStats_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "ActorRoles" (
    "ActorId" TEXT NOT NULL,
    "RoleId" TEXT NOT NULL,
    CONSTRAINT "PK_ActorRoles" PRIMARY KEY ("ActorId", "RoleId"),
    CONSTRAINT "FK_ActorRoles_Actors_ActorId" FOREIGN KEY ("ActorId") REFERENCES "Actors" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_ActorRoles_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Permissions" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Permissions" PRIMARY KEY,
    "ResourceAction" TEXT NOT NULL,
    "ResourceType" TEXT NOT NULL,
    "RoleId" TEXT NOT NULL,
    CONSTRAINT "FK_Permissions_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "AlertEvents" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_AlertEvents" PRIMARY KEY,
    "AcknowledgedAt" TEXT NULL,
    "AcknowledgedByActorId" TEXT NULL,
    "AlertRuleId" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "DeduplicationKey" TEXT NOT NULL,
    "Info" TEXT NOT NULL,
    "OpenIncidentKey" TEXT NULL,
    "ResolutionNote" TEXT NULL,
    "ResolvedAt" TEXT NULL,
    "ResolvedByActorId" TEXT NULL,
    "ResourceId" TEXT NULL,
    "ResourceName" TEXT NOT NULL,
    "ResourceType" TEXT NOT NULL,
    "Severity" TEXT NOT NULL,
    "Type" TEXT NOT NULL,
    "UpdatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT "FK_AlertEvents_AlertRules_AlertRuleId" FOREIGN KEY ("AlertRuleId") REFERENCES "AlertRules" ("Id") ON DELETE CASCADE
);

CREATE TABLE "AlertRuleChannels" (
    "AlertRuleId" TEXT NOT NULL,
    "AlertChannelId" TEXT NOT NULL,
    CONSTRAINT "PK_AlertRuleChannels" PRIMARY KEY ("AlertRuleId", "AlertChannelId"),
    CONSTRAINT "FK_AlertRuleChannels_AlertChannels_AlertChannelId" FOREIGN KEY ("AlertChannelId") REFERENCES "AlertChannels" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_AlertRuleChannels_AlertRules_AlertRuleId" FOREIGN KEY ("AlertRuleId") REFERENCES "AlertRules" ("Id") ON DELETE CASCADE
);

CREATE TABLE "AlertRuleStates" (
    "AlertRuleId" TEXT NOT NULL,
    "ResourceId" TEXT NOT NULL,
    "ConsecutiveMatches" INTEGER NOT NULL DEFAULT 3,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "LastTriggeredAt" TEXT NULL,
    CONSTRAINT "PK_AlertRuleStates" PRIMARY KEY ("AlertRuleId", "ResourceId"),
    CONSTRAINT "FK_AlertRuleStates_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_AlertRuleStates_AlertRules_AlertRuleId" FOREIGN KEY ("AlertRuleId") REFERENCES "AlertRules" ("Id") ON DELETE CASCADE
);

CREATE TABLE "GitRepositories" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_GitRepositories" PRIMARY KEY,
    "ControlStartedAt" INTEGER NULL,
    "ControlState" TEXT NULL DEFAULT 'Idle',
    "ControlTriggeredBy" TEXT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "DefaultBranch" TEXT NOT NULL,
    "Description" TEXT NULL,
    "GitAccountId" TEXT NULL,
    "Name" TEXT NOT NULL,
    "OnClone" TEXT NULL,
    "OnPull" TEXT NULL,
    "RowVersion" INTEGER NOT NULL DEFAULT 0,
    "Status" TEXT NOT NULL,
    "Url" TEXT NOT NULL,
    "WebHookEnabled" INTEGER NOT NULL DEFAULT 0,
    "WebHookSecret" TEXT NULL,
    CONSTRAINT "FK_GitRepositories_Actors_ControlTriggeredBy" FOREIGN KEY ("ControlTriggeredBy") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_GitRepositories_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_GitRepositories_GitAccounts_GitAccountId" FOREIGN KEY ("GitAccountId") REFERENCES "GitAccounts" ("Id") ON DELETE SET NULL
);

CREATE TABLE "Images" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Images" PRIMARY KEY,
    "Containers" INTEGER NOT NULL DEFAULT 0,
    "ControlStartedAt" INTEGER NULL,
    "ControlState" TEXT NULL DEFAULT 'Idle',
    "ControlTriggeredBy" TEXT NULL,
    "CreatedAt" TEXT NOT NULL,
    "DockerImageId" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "RegistryId" TEXT NULL,
    "RowVersion" INTEGER NOT NULL DEFAULT 0,
    "Size" REAL NOT NULL DEFAULT 0.0,
    "Tags" TEXT NOT NULL,
    "UpdatedAt" TEXT NULL,
    CONSTRAINT "FK_Images_Actors_ControlTriggeredBy" FOREIGN KEY ("ControlTriggeredBy") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Images_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_Images_Registries_RegistryId" FOREIGN KEY ("RegistryId") REFERENCES "Registries" ("Id") ON DELETE SET NULL
);

CREATE TABLE "StackReleases" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_StackReleases" PRIMARY KEY,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "Spec" TEXT NOT NULL,
    "StackId" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    "Version" TEXT NOT NULL,
    CONSTRAINT "FK_StackReleases_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_StackReleases_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_StackReleases_Stacks_StackId" FOREIGN KEY ("StackId") REFERENCES "Stacks" ("Id") ON DELETE CASCADE
);

CREATE TABLE "RefreshTokens" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_RefreshTokens" PRIMARY KEY,
    "CreatedAt" TEXT NOT NULL,
    "UserId" TEXT NOT NULL,
    CONSTRAINT "FK_RefreshTokens_Users_UserId" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE
);

CREATE TABLE "UsersTeams" (
    "UserId" TEXT NOT NULL,
    "TeamId" TEXT NOT NULL,
    CONSTRAINT "PK_UsersTeams" PRIMARY KEY ("UserId", "TeamId"),
    CONSTRAINT "FK_UsersTeams_Teams_TeamId" FOREIGN KEY ("TeamId") REFERENCES "Teams" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_UsersTeams_Users_UserId" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Containers" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Containers" PRIMARY KEY,
    "ControlStartedAt" INTEGER NULL,
    "ControlState" TEXT NULL DEFAULT 'Idle',
    "ControlTriggeredBy" TEXT NULL,
    "Created" REAL NOT NULL,
    "DeploymentId" TEXT NULL,
    "DockerContainerId" TEXT NOT NULL,
    "DockerImageId" TEXT NOT NULL,
    "ImageId" TEXT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "Ports" TEXT NOT NULL,
    "RowVersion" INTEGER NOT NULL DEFAULT 0,
    "Stack" TEXT NULL,
    "State" TEXT NOT NULL,
    "Updated" TEXT NOT NULL,
    CONSTRAINT "FK_Containers_Actors_ControlTriggeredBy" FOREIGN KEY ("ControlTriggeredBy") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Containers_Deployments_DeploymentId" FOREIGN KEY ("DeploymentId") REFERENCES "Deployments" ("Id") ON DELETE SET NULL,
    CONSTRAINT "FK_Containers_Images_ImageId" FOREIGN KEY ("ImageId") REFERENCES "Images" ("Id") ON DELETE SET NULL,
    CONSTRAINT "FK_Containers_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "ContainerStats" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_ContainerStats" PRIMARY KEY,
    "ContainerId" TEXT NOT NULL,
    "CpuUsage" REAL NOT NULL,
    "Created" REAL NOT NULL,
    "MemoryActive" REAL NOT NULL,
    "MemoryCache" REAL NOT NULL,
    "MemoryLimit" REAL NOT NULL,
    "RxBytes" REAL NOT NULL,
    "TxBytes" REAL NOT NULL,
    CONSTRAINT "FK_ContainerStats_Containers_ContainerId" FOREIGN KEY ("ContainerId") REFERENCES "Containers" ("Id") ON DELETE CASCADE
);

INSERT INTO "Actors" ("Id", "Type")
VALUES ('00000000-0000-0000-0000-000000000001', 'System');
SELECT changes();

INSERT INTO "Actors" ("Id", "Type")
VALUES ('00000000-0000-0000-0000-000000000002', 'User');
SELECT changes();

INSERT INTO "Actors" ("Id", "Type")
VALUES ('00000000-0000-0000-0000-000000000003', 'Team');
SELECT changes();


INSERT INTO "Roles" ("Id", "Name")
VALUES ('30000000-0000-0000-0000-000000000001', 'Admin');
SELECT changes();

INSERT INTO "Roles" ("Id", "Name")
VALUES ('30000000-0000-0000-0000-000000000002', 'Operator');
SELECT changes();

INSERT INTO "Roles" ("Id", "Name")
VALUES ('30000000-0000-0000-0000-000000000003', 'Viewer');
SELECT changes();


INSERT INTO "ActorRoles" ("ActorId", "RoleId")
VALUES ('00000000-0000-0000-0000-000000000002', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "ActorRoles" ("ActorId", "RoleId")
VALUES ('00000000-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000002');
SELECT changes();


INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000001', 300, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'CPU > 90% - Platform', '[]', 3, 'Critical', 90.0, 'PlatformCpuHigh');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000002', 300, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'RAM > 90% - Platform', '[]', 3, 'Critical', 90.0, 'PlatformRamHigh');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000003', 600, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Platform Unreachable', '[]', NULL, 'Critical', NULL, 'PlatformUnreachable');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000004', 3600, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Platform Version Mismatch', '[]', NULL, 'Warning', NULL, 'PlatformVersionMismatch');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000005', NULL, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Unmanaged Container Created', '[]', NULL, 'Info', NULL, 'UnmanagedContainerCreated');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000006', 86400, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Image Update Available - Deployment', '[]', NULL, 'Info', NULL, 'DeploymentImageUpdateAvailable');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000007', NULL, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Auto Deploy Failed - Deployment', '[]', NULL, 'Critical', NULL, 'DeploymentAutoDeployFailed');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000008', NULL, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Deployment Auto Updated', '[]', NULL, 'Info', NULL, 'DeploymentAutoUpdated');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000009', 86400, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Image Update Available - Stack', '[]', NULL, 'Info', NULL, 'StackImageUpdateAvailable');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-00000000000a', NULL, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Auto Deploy Failed - Stack', '[]', NULL, 'Critical', NULL, 'StackAutoDeployFailed');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-00000000000b', NULL, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Stack Auto Updated', '[]', NULL, 'Info', NULL, 'StackAutoUpdated');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000011', 300, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'CPU > 80% - Platform', '[]', 3, 'Warning', 80.0, 'PlatformCpuHigh');
SELECT changes();

INSERT INTO "AlertRules" ("Id", "CooldownSeconds", "CreatedAt", "CreatedByActorId", "Description", "LimitedTo", "Name", "QuietHours", "RequiredMatches", "Severity", "Threshold", "Type")
VALUES ('019d0000-0001-7000-8001-000000000022', 300, '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'RAM > 80% - Platform', '[]', 3, 'Warning', 80.0, 'PlatformRamHigh');
SELECT changes();


INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('017b5a65-e312-8a67-b7fd-6124f6ddeeee', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('052e45fb-cb15-9380-6a33-c233fde703a6', 'Update', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('0dd6ded1-5ef7-c1b5-a36a-d0de73459d8a', 'Apply', 'Registry', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('0eda225f-cb5b-1bb0-9525-be92b14fc322', 'Apply', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('0f030749-53d1-bade-8ef3-30112991786d', 'Create', 'Stack', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('117176b6-ca23-e53d-d996-83affab7ed48', 'View', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('12bbcf07-7237-3afb-65f7-1fc8e2de4939', 'Apply', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('18255099-e963-802b-21a3-d115440e9322', 'Pull', 'Deployment', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('18b8b740-528c-6366-9902-ebf6a025d063', 'Apply', 'Alert', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('19357866-b0f7-4c0b-fb01-c3a6556d1e5f', 'Apply', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('1a0b6504-d508-0f6d-4513-12b80c3ab4d8', 'Update', 'Platform', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('1f00afd4-a4d1-94cd-3a94-44d420eef066', 'Apply', 'GitRepository', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('21d8c7f7-5480-5509-e2ab-e3c8fdbb5ab8', 'Update', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('2548763c-c9b7-5359-a80a-5ce706c5c42c', 'Update', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('258c5870-adb0-f28f-bed2-5c993e5d11da', 'Pull', 'GitRepository', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('26bb8bc9-e526-dfd5-c3cc-2ebc0fb9837b', 'Apply', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('2e0582c8-9569-22cd-c777-69f03893b8ec', 'Delete', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('30f18293-2d41-2525-3210-e0f83bafa13d', 'Update', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('3a8c08b1-d033-1580-65f9-a1cb3ed3fc6a', 'Create', 'Registry', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('3d84b4f0-2433-c34e-45e1-84d18b6c155d', 'Pull', 'Platform', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('3e5365c6-7759-a0ad-e7fa-32263d00682c', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('40aadb71-124b-7c1b-44b9-f507a69ade11', 'Pull', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('4711987b-af34-12f7-4cf4-795f51049571', 'Pull', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('47c763f4-71e9-2992-3223-8ab97876b727', 'Apply', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('4aedeb0a-de43-b841-5970-9743bcde952b', 'Create', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('4b68a9cf-0af5-b0d6-8c05-e8b7e98b3919', 'View', 'Stack', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('4cfe0dcb-ce92-0500-981f-7d79b3782877', 'Create', 'Alert', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('4fa196f3-a9e5-7716-b1dd-aa574061e1f7', 'Pull', 'GitAccount', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('55228106-7ae9-6748-5a33-73e253ad940d', 'Apply', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('58e018b1-ba4c-56bb-c56c-b9473688127b', 'Pull', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('5e8afc50-270c-4413-c5f1-bd25dac435d9', 'Apply', 'Stack', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('5ede29a8-8e33-2d7c-48c3-1e5d733edd73', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('613e9000-da2c-b4e7-9e02-b4bef349f0f7', 'Pull', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('62e8e1fe-c911-e1b9-cc70-7efff6e08327', 'Pull', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('6395043d-510b-dc85-f19d-2a57463f4e8f', 'Pull', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('65cb87df-133d-b577-21f6-2f20190ce45f', 'View', 'Registry', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('6deaa9b4-66e6-22bb-3f49-62584ccd9e1f', 'View', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('6e332b06-35e2-fbbd-e12c-bb7f02ab2474', 'Create', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('6ed1c4e3-9d28-23d8-2939-6a414aa0f53d', 'Apply', 'Platform', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('6f3a7126-e96a-d249-5e58-108bd0bd1f0f', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('7238045c-c070-0ba5-b133-6083ea208d1b', 'Apply', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('73174250-b459-3def-d4e6-82d09d07ece9', 'Update', 'GitRepository', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('75575fd4-2d45-4302-12ba-1ebf9e9ea17f', 'Create', 'GitRepository', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('780d5066-5b19-668e-9f2f-0103f6cb23be', 'View', 'Deployment', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('783ca30a-d153-8f1b-27db-c3e722f7f34d', 'Apply', 'GitAccount', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('7ad3c461-3f87-fe3c-a1e7-4a490906600e', 'Delete', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('7ba78b50-a388-1606-e334-30c69758e60f', 'View', 'Deployment', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('7ccb3b9e-a09a-d3c9-82ed-3f71646d2576', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('7fddc3c2-6d6e-cb92-a7a3-59a7a18b7c08', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('83ac5b37-8bd4-e093-3cdd-7e9f61300108', 'View', 'Alert', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('8658afec-8be0-2f4b-7b1e-478ac44341ec', 'Create', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('88dc9733-349d-635c-7ef6-829065f4f87b', 'Create', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('8b1633bc-d6ba-a419-38ea-e4d7e4b48cbe', 'Create', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('8ce09606-e435-8a9b-2dab-8d1dfc91a198', 'Delete', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('8de4cd72-b2ce-4e18-f148-b93892845825', 'View', 'Alert', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('9365df99-cab8-01da-f3c7-dc17a54e8801', 'Delete', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('961c1641-93aa-54ca-9b00-6608da3ae4c8', 'Pull', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('969a37a2-af1e-fa24-35b8-4857f802e001', 'Apply', 'Deployment', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('99a0ba24-900d-448e-70f0-6e5eda10f8fc', 'Apply', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('99b23eff-a5c8-0cbe-6383-b99e43a43b23', 'Create', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('9e087c4f-e933-37c9-3941-b1803f83ede3', 'Create', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('9f3d1be4-d19e-9453-86aa-2ae06eae6d18', 'Delete', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('9fd296b8-cb43-5a62-9672-cf562aa6efd1', 'Update', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('a178007d-0c14-258e-bb6a-8828a5c28db7', 'Update', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('a1a791a5-9c37-88ad-c0e2-9a6717191298', 'Update', 'Stack', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('acab0152-cf67-d16c-fe1e-c579972ad2df', 'Pull', 'Registry', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('acece9ff-20d1-f3d5-c304-3a07adb9a03b', 'Update', 'Alert', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('b678aa42-8c01-b706-1332-b85adb4e3096', 'Delete', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('bf01fa5c-a0a9-749e-b6c9-2d04af953b2b', 'Create', 'GitRepository', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('c1184544-a092-3f80-c4d6-6f778db56b26', 'Update', 'GitAccount', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('c12c9075-9343-73ec-322a-cc41a23230db', 'Delete', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('c3e7230a-af2b-ba29-57ae-3c4043b66d61', 'View', 'Deployment', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('c71643e0-9549-edeb-c7ff-496efa58260c', 'Create', 'Platform', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('c717528a-5a83-69a2-6893-4ab5fbc14d1b', 'View', 'Platform', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('cdd4e750-840f-2a08-7a9a-3bd65a4360e9', 'Delete', 'Stack', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('ce566660-f041-32b6-0942-2f8abb92b17d', 'View', 'Registry', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('d23893f2-7f44-a593-83de-22ca4a8c80a1', 'Update', 'Registry', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('d5ba4edc-5278-a21e-613d-91350e52bce7', 'Apply', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('d868c269-b60f-2be0-2451-9b81b3c95674', 'View', 'AlertChannel', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('d990b800-123d-a0ec-9b6a-239915235880', 'Create', 'GitAccount', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('df77eb4e-7860-5319-431e-481bfe08baeb', 'Pull', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('e08e5c0a-2e94-f112-22dd-e06d44dd2d9b', 'View', 'Stack', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('e13e141d-8322-f5f5-0488-cf961e919143', 'Update', 'Deployment', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('e3b44e1b-f776-b5ce-509d-2b529768a528', 'View', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('e6a62042-68c0-d60f-2888-f685176a8f4f', 'Create', 'Deployment', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('e92ef59e-f1ab-51fc-abc8-4d90c98e5bf9', 'Update', 'Platform', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('ea5f48d2-2719-0a78-dcb4-2efd447e674f', 'Pull', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('ec427e29-a8ed-8604-59cc-7eda3268fc30', 'View', 'Alert', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f12e902b-29c0-d404-41ef-6c7731211a70', 'Pull', 'Stack', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f1782e79-808e-d628-a83a-10b4639b9a68', 'View', 'GitRepository', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f30ff44f-86d5-8215-2c0f-60ed6ebd6234', 'Update', 'Registry', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f4595acc-f991-34d8-834c-4a1136010e17', 'View', 'Platform', '30000000-0000-0000-0000-000000000003');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f7be80a4-dffa-1049-994a-5314ee03efaf', 'Create', 'GitAccount', '30000000-0000-0000-0000-000000000001');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('f9af8f42-cf9c-21a8-43ba-793b4dd1bd3f', 'Pull', 'Alert', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('fc6bc1bc-bb69-098b-b09e-07a96444f57e', 'Update', 'AlertChannel', '30000000-0000-0000-0000-000000000002');
SELECT changes();

INSERT INTO "Permissions" ("Id", "ResourceAction", "ResourceType", "RoleId")
VALUES ('ff6e9bea-dbf2-e811-d4ba-6702a55c3f92', 'View', 'GitAccount', '30000000-0000-0000-0000-000000000002');
SELECT changes();


INSERT INTO "Registries" ("Id", "Configuration", "CreatedAt", "CreatedByActorId", "Description", "Name", "RegistryHost", "Status")
VALUES ('00000000-0000-0000-0000-000000000100', (('{' || (CHAR(13) || CHAR(10))) || (('    "$type": "DockerHub"' || CHAR(13)) || (CHAR(10) || '}'))), '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', 'Public Docker Hub Registry', 'Docker Hub', 'hub.docker.com', 'Active');
SELECT changes();


INSERT INTO "Teams" ("Id", "ActorId", "Name")
VALUES ('20000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000003', 'Default Team');
SELECT changes();


INSERT INTO "Users" ("Id", "ActorId", "CreatedAt", "CreatedByActorId", "Email", "Name", "Password")
VALUES ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002', '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', 'admin@citadel.local', 'Admin', 'o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1');
SELECT changes();


CREATE INDEX "IX_ActivityEvents_CreatedByActorId" ON "ActivityEvents" ("CreatedByActorId");

CREATE INDEX "IX_ActivityEvents_EventType" ON "ActivityEvents" ("EventType");

CREATE INDEX "IX_ActivityEvents_Platform_CreatedAt" ON "ActivityEvents" ("PlatformId", "CreatedAt");

CREATE INDEX "IX_ActivityEvents_Resource_CreatedAt" ON "ActivityEvents" ("ResourceId", "CreatedAt");

CREATE INDEX "IX_ActivityEvents_Status" ON "ActivityEvents" ("Status");

CREATE INDEX "IX_ActorRoles_ActorId" ON "ActorRoles" ("ActorId");

CREATE INDEX "IX_ActorRoles_RoleId" ON "ActorRoles" ("RoleId");

CREATE INDEX "IX_AlertChannels_CreatedByActorId" ON "AlertChannels" ("CreatedByActorId");

CREATE INDEX "IX_AlertEvents_AlertRuleId" ON "AlertEvents" ("AlertRuleId");

CREATE UNIQUE INDEX "IX_AlertEvents_OpenIncidentKey" ON "AlertEvents" ("OpenIncidentKey");

CREATE INDEX "IX_AlertEvents_ResourceType" ON "AlertEvents" ("ResourceType");

CREATE INDEX "IX_AlertEvents_Resource_CreatedAt" ON "AlertEvents" ("ResourceId", "CreatedAt");

CREATE INDEX "IX_AlertEvents_Type" ON "AlertEvents" ("Type");

CREATE INDEX "IX_AlertRuleChannels_AlertChannelId" ON "AlertRuleChannels" ("AlertChannelId");

CREATE INDEX "IX_AlertRuleStates_CreatedByActorId" ON "AlertRuleStates" ("CreatedByActorId");

CREATE INDEX "IX_AlertRules_CreatedByActorId" ON "AlertRules" ("CreatedByActorId");

CREATE INDEX "IX_AlertRules_Type" ON "AlertRules" ("Type");

CREATE UNIQUE INDEX "IX_ContainerStats_ContainerId_Created" ON "ContainerStats" ("ContainerId", "Created");

CREATE INDEX "IX_Containers_ControlTriggeredBy" ON "Containers" ("ControlTriggeredBy");

CREATE INDEX "IX_Containers_DeploymentId" ON "Containers" ("DeploymentId");

CREATE INDEX "IX_Containers_DockerImageId" ON "Containers" ("DockerImageId");

CREATE INDEX "IX_Containers_ImageId" ON "Containers" ("ImageId");

CREATE INDEX "IX_Containers_PlatformId" ON "Containers" ("PlatformId");

CREATE UNIQUE INDEX "IX__Containers_DockerContainerId_PlatformId" ON "Containers" ("DockerContainerId", "PlatformId");

CREATE INDEX "IX_Deployments_ControlTriggeredBy" ON "Deployments" ("ControlTriggeredBy");

CREATE INDEX "IX_Deployments_CreatedByActorId" ON "Deployments" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Deployments_Name_PlatformId" ON "Deployments" ("Name", "PlatformId");

CREATE INDEX "IX_Deployments_PlatformId" ON "Deployments" ("PlatformId");

CREATE INDEX "IX_GitAccounts_CreatedByActorId" ON "GitAccounts" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_GitAccounts_Name" ON "GitAccounts" ("Name");

CREATE INDEX "IX_GitRepositories_ControlTriggeredBy" ON "GitRepositories" ("ControlTriggeredBy");

CREATE INDEX "IX_GitRepositories_CreatedByActorId" ON "GitRepositories" ("CreatedByActorId");

CREATE INDEX "IX_GitRepositories_GitAccountId" ON "GitRepositories" ("GitAccountId");

CREATE UNIQUE INDEX "IX_GitRepositories_Name" ON "GitRepositories" ("Name");

CREATE INDEX "IX_Images_ControlTriggeredBy" ON "Images" ("ControlTriggeredBy");

CREATE UNIQUE INDEX "IX_Images_DockerImageId_PlatformId" ON "Images" ("DockerImageId", "PlatformId");

CREATE INDEX "IX_Images_PlatformId" ON "Images" ("PlatformId");

CREATE INDEX "IX_Images_RegistryId" ON "Images" ("RegistryId");

CREATE INDEX "IX_Permissions_RoleId" ON "Permissions" ("RoleId");

CREATE UNIQUE INDEX "IX_PlatformStats_PlatformId_Created" ON "PlatformStats" ("PlatformId", "Created");

CREATE UNIQUE INDEX "IX_Platforms_Address" ON "Platforms" ("Address");

CREATE INDEX "IX_RefreshTokens_UserId" ON "RefreshTokens" ("UserId");

CREATE INDEX "IX_Registries_CreatedByActorId" ON "Registries" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Registries_Name" ON "Registries" ("Name");

CREATE INDEX "IX_ResourceAccesses_Actor" ON "ResourceAccesses" ("ActorId");

CREATE INDEX "IX_ResourceAccesses_ResourceType_ResourceId_ActorId" ON "ResourceAccesses" ("ResourceType", "ResourceId", "ActorId");

CREATE UNIQUE INDEX "IX_ResourceAccesses_ResourceType_ResourceId_ActorId_Action" ON "ResourceAccesses" ("ResourceType", "ResourceId", "ActorId", "Action");

CREATE INDEX "IX_StackReleases_CreatedByActorId" ON "StackReleases" ("CreatedByActorId");

CREATE INDEX "IX_StackReleases_PlatformId" ON "StackReleases" ("PlatformId");

CREATE INDEX "IX_StackReleases_StackId" ON "StackReleases" ("StackId");

CREATE INDEX "IX_Stacks_ControlTriggeredBy" ON "Stacks" ("ControlTriggeredBy");

CREATE INDEX "IX_Stacks_CreatedByActorId" ON "Stacks" ("CreatedByActorId");

CREATE INDEX "IX_Stacks_CurrentStackReleaseId" ON "Stacks" ("CurrentStackReleaseId");

CREATE UNIQUE INDEX "IX_Teams_ActorId" ON "Teams" ("ActorId");

CREATE UNIQUE INDEX "IX_Users_ActorId" ON "Users" ("ActorId");

CREATE INDEX "IX_Users_CreatedByActorId" ON "Users" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Users_Email" ON "Users" ("Email");

CREATE INDEX "IX_UsersTeams_TeamId" ON "UsersTeams" ("TeamId");

CREATE INDEX "IX_UsersTeams_UserId" ON "UsersTeams" ("UserId");

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260406095843_migration0001', '10.0.5');

COMMIT;

