CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
    "ProductVersion" TEXT NOT NULL
);

BEGIN TRANSACTION;
CREATE TABLE "Actors" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Actors" PRIMARY KEY,
    "Name" TEXT NOT NULL,
    "Type" TEXT NOT NULL
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

CREATE TABLE "Roles" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Roles" PRIMARY KEY,
    "CreatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00',
    "Name" TEXT NOT NULL,
    "UpdatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00'
);

CREATE TABLE "Registries" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Registries" PRIMARY KEY,
    "Configuration" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    "CreatedByActorId" TEXT NOT NULL,
    "Description" TEXT NULL,
    "Name" TEXT NOT NULL,
    "RegistryHost" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    CONSTRAINT "FK_Registries_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT
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

CREATE TABLE "Deployments" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Deployments" PRIMARY KEY,
    "AutoUpdateState_CurrentDigest" TEXT NULL,
    "AutoUpdateState_LastCheckedAt" TEXT NULL,
    "AutoUpdateState_LastError" TEXT NULL,
    "AutoUpdateState_RemoteDigest" TEXT NULL,
    "AutoUpdateState_Status" TEXT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    "CreatedByActorId" TEXT NOT NULL,
    "Description" TEXT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "Spec" TEXT NOT NULL,
    "Status" TEXT NOT NULL,
    "UpdateBehavior" TEXT NULL,
    CONSTRAINT "FK_Deployments_Actors_CreatedByActorId" FOREIGN KEY ("CreatedByActorId") REFERENCES "Actors" ("Id") ON DELETE RESTRICT,
    CONSTRAINT "FK_Deployments_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
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

CREATE TABLE "Permissions" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Permissions" PRIMARY KEY,
    "PermissionCode" TEXT NOT NULL,
    "RoleId" TEXT NOT NULL,
    CONSTRAINT "FK_Permissions_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Teams" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Teams" PRIMARY KEY,
    "Name" TEXT NOT NULL,
    "RoleId" TEXT NOT NULL,
    CONSTRAINT "FK_Teams_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Images" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Images" PRIMARY KEY,
    "Containers" INTEGER NOT NULL DEFAULT 0,
    "CreatedAt" TEXT NOT NULL,
    "DockerImageId" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "RegistryId" TEXT NULL,
    "Size" REAL NOT NULL DEFAULT 0.0,
    "Tags" TEXT NOT NULL,
    "UpdatedAt" TEXT NULL,
    CONSTRAINT "FK_Images_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_Images_Registries_RegistryId" FOREIGN KEY ("RegistryId") REFERENCES "Registries" ("Id") ON DELETE SET NULL
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
    "Created" REAL NOT NULL,
    "DeploymentId" TEXT NULL,
    "DockerContainerId" TEXT NOT NULL,
    "DockerImageId" TEXT NOT NULL,
    "ImageId" TEXT NULL,
    "Name" TEXT NOT NULL,
    "PlatformId" TEXT NOT NULL,
    "Ports" TEXT NOT NULL,
    "Stack" TEXT NULL,
    "State" TEXT NOT NULL,
    "Updated" TEXT NOT NULL,
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

INSERT INTO "Actors" ("Id", "Name", "Type")
VALUES ('00000000-0000-0000-0000-000000000001', 'System', 'System');
SELECT changes();

INSERT INTO "Actors" ("Id", "Name", "Type")
VALUES ('00000000-0000-0000-0000-000000000002', 'Admin', 'User');
SELECT changes();


INSERT INTO "Roles" ("Id", "CreatedAt", "Name", "UpdatedAt")
VALUES ('bdde9601-3b03-1275-a11b-98533d063a04', '2026-01-01 00:00:00', 'Admin', '2026-01-01 00:00:00');
SELECT changes();


INSERT INTO "Registries" ("Id", "Configuration", "CreatedAt", "CreatedByActorId", "Description", "Name", "RegistryHost", "Status")
VALUES ('a1de9601-7f3b-4f75-a11b-98533d063a0f', (('{' || (CHAR(13) || CHAR(10))) || (('    "$type": "DockerHub"' || CHAR(13)) || (CHAR(10) || '}'))), '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', 'Public Docker Hub Registry', 'Docker Hub', 'hub.docker.com', 'Active');
SELECT changes();


INSERT INTO "Teams" ("Id", "Name", "RoleId")
VALUES ('cede9601-67e9-507d-832c-0ca0155465a1', 'Admins', 'bdde9601-3b03-1275-a11b-98533d063a04');
SELECT changes();


INSERT INTO "Users" ("Id", "ActorId", "CreatedAt", "CreatedByActorId", "Email", "Name", "Password")
VALUES ('d1de9601-f113-ce77-884e-3cb636ec09a8', '00000000-0000-0000-0000-000000000002', '2026-01-01 00:00:00', '00000000-0000-0000-0000-000000000001', 'admin@admin.com', 'admin', 'o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1');
SELECT changes();


INSERT INTO "UsersTeams" ("TeamId", "UserId")
VALUES ('cede9601-67e9-507d-832c-0ca0155465a1', 'd1de9601-f113-ce77-884e-3cb636ec09a8');
SELECT changes();


CREATE UNIQUE INDEX "IX_ContainerStats_ContainerId_Created" ON "ContainerStats" ("ContainerId", "Created");

CREATE INDEX "IX_Containers_DeploymentId" ON "Containers" ("DeploymentId");

CREATE INDEX "IX_Containers_DockerImageId" ON "Containers" ("DockerImageId");

CREATE INDEX "IX_Containers_ImageId" ON "Containers" ("ImageId");

CREATE INDEX "IX_Containers_PlatformId" ON "Containers" ("PlatformId");

CREATE UNIQUE INDEX "IX__Containers_DockerContainerId_PlatformId" ON "Containers" ("DockerContainerId", "PlatformId");

CREATE INDEX "IX_Deployments_CreatedByActorId" ON "Deployments" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Deployments_Name" ON "Deployments" ("Name");

CREATE INDEX "IX_Deployments_PlatformId" ON "Deployments" ("PlatformId");

CREATE UNIQUE INDEX "IX_Images_DockerImageId_PlatformId" ON "Images" ("DockerImageId", "PlatformId");

CREATE INDEX "IX_Images_PlatformId" ON "Images" ("PlatformId");

CREATE INDEX "IX_Images_RegistryId" ON "Images" ("RegistryId");

CREATE INDEX "IX_Permissions_RoleId" ON "Permissions" ("RoleId");

CREATE UNIQUE INDEX "IX_PlatformStats_PlatformId_Created" ON "PlatformStats" ("PlatformId", "Created");

CREATE UNIQUE INDEX "IX_Platforms_Address" ON "Platforms" ("Address");

CREATE INDEX "IX_RefreshTokens_UserId" ON "RefreshTokens" ("UserId");

CREATE INDEX "IX_Registries_CreatedByActorId" ON "Registries" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Registries_Name" ON "Registries" ("Name");

CREATE INDEX "IX_Teams_RoleId" ON "Teams" ("RoleId");

CREATE UNIQUE INDEX "IX_Users_ActorId" ON "Users" ("ActorId");

CREATE INDEX "IX_Users_CreatedByActorId" ON "Users" ("CreatedByActorId");

CREATE UNIQUE INDEX "IX_Users_Email" ON "Users" ("Email");

CREATE INDEX "IX_UsersTeams_TeamId" ON "UsersTeams" ("TeamId");

CREATE INDEX "IX_UsersTeams_UserId" ON "UsersTeams" ("UserId");

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20251228120950_migration0001', '10.0.1');

COMMIT;

