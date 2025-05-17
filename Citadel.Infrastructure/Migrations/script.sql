CREATE TABLE IF NOT EXISTS "__EFMigrationsHistory" (
    "MigrationId" TEXT NOT NULL CONSTRAINT "PK___EFMigrationsHistory" PRIMARY KEY,
    "ProductVersion" TEXT NOT NULL
);

BEGIN TRANSACTION;
CREATE TABLE "Platforms" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Platforms" PRIMARY KEY,
    "Name" TEXT NULL,
    "Address" TEXT NULL,
    "DaemonId" TEXT NULL,
    "Status" TEXT NOT NULL,
    "NetworksCount" INTEGER NOT NULL,
    "VolumesCount" INTEGER NOT NULL,
    "Containers" INTEGER NOT NULL,
    "ContainersRunning" INTEGER NOT NULL,
    "ContainersPaused" INTEGER NOT NULL,
    "ContainersStopped" INTEGER NOT NULL,
    "Images" INTEGER NOT NULL,
    "Driver" TEXT NULL,
    "OperatingSystem" TEXT NULL,
    "OsVersion" TEXT NULL,
    "OsType" TEXT NULL,
    "Architecture" TEXT NULL,
    "Ncpu" INTEGER NOT NULL,
    "MemTotal" INTEGER NOT NULL,
    "ServerVersion" TEXT NULL,
    "AgentVersion" TEXT NULL
);

CREATE TABLE "Registries" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Registries" PRIMARY KEY,
    "Name" TEXT NOT NULL,
    "Url" TEXT NOT NULL,
    "Created" TEXT NOT NULL,
    "Discriminator" TEXT NOT NULL,
    "Configuration" TEXT NOT NULL
);

CREATE TABLE "Roles" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Roles" PRIMARY KEY,
    "Name" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT '2025-05-17 15:18:55.8493047',
    "UpdatedAt" TEXT NOT NULL DEFAULT '2025-05-17 15:18:55.8507004'
);

CREATE TABLE "Users" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Users" PRIMARY KEY,
    "Name" TEXT NULL,
    "Email" TEXT NOT NULL,
    "Password" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL DEFAULT '2025-05-17 15:18:55.862409',
    "UpdatedAt" TEXT NOT NULL DEFAULT '2025-05-17 15:18:55.8624991'
);

CREATE TABLE "ContainersInfo" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_ContainersInfo" PRIMARY KEY,
    "PlatformId" TEXT NOT NULL,
    "ContainerId" TEXT NULL,
    "Name" TEXT NULL,
    "Image" TEXT NULL,
    "Created" INTEGER NOT NULL,
    "State" TEXT NOT NULL,
    "Stack" TEXT NULL,
    "Status" TEXT NULL,
    "Ports" TEXT NULL,
    CONSTRAINT "FK_ContainersInfo_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "PlatformStats" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_PlatformStats" PRIMARY KEY,
    "PlatformId" TEXT NOT NULL,
    "Created" INTEGER NOT NULL,
    "MemoryUsage" REAL NOT NULL,
    "CpuUsage" REAL NOT NULL,
    "RxBytes" REAL NOT NULL,
    "TxBytes" REAL NOT NULL,
    CONSTRAINT "FK_PlatformStats_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "SwarmsInfo" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_SwarmsInfo" PRIMARY KEY,
    "PlatformId" TEXT NOT NULL,
    "NodeID" TEXT NULL,
    "NodeAddr" TEXT NULL,
    "LocalNodeState" TEXT NULL,
    "ControlAvailable" INTEGER NOT NULL,
    "Error" TEXT NULL,
    "Nodes" INTEGER NOT NULL,
    "Managers" INTEGER NOT NULL,
    CONSTRAINT "FK_SwarmsInfo_Platforms_PlatformId" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Permissions" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Permissions" PRIMARY KEY,
    "RoleId" TEXT NOT NULL,
    "PermissionCode" INTEGER NOT NULL,
    CONSTRAINT "FK_Permissions_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "Teams" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_Teams" PRIMARY KEY,
    "RoleId" TEXT NOT NULL,
    "Name" TEXT NOT NULL,
    CONSTRAINT "FK_Teams_Roles_RoleId" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE
);

CREATE TABLE "RefreshTokens" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_RefreshTokens" PRIMARY KEY,
    "UserId" TEXT NOT NULL,
    "CreatedAt" TEXT NOT NULL,
    CONSTRAINT "FK_RefreshTokens_Users_UserId" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE
);

CREATE TABLE "ContainerStats" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_ContainerStats" PRIMARY KEY,
    "ContainerInfoId" TEXT NOT NULL,
    "Created" INTEGER NOT NULL,
    "MemoryUsage" REAL NULL,
    "CpuUsage" REAL NULL,
    "MemoryLimit" REAL NULL,
    "RxBytes" INTEGER NULL,
    "TxBytes" INTEGER NULL,
    CONSTRAINT "FK_ContainerStats_ContainersInfo_ContainerInfoId" FOREIGN KEY ("ContainerInfoId") REFERENCES "ContainersInfo" ("Id") ON DELETE CASCADE
);

CREATE TABLE "SwarmsPeer" (
    "Id" TEXT NOT NULL CONSTRAINT "PK_SwarmsPeer" PRIMARY KEY,
    "SwarmInfoId" TEXT NOT NULL,
    "NodeID" TEXT NULL,
    "Addr" TEXT NULL,
    CONSTRAINT "FK_SwarmsPeer_SwarmsInfo_SwarmInfoId" FOREIGN KEY ("SwarmInfoId") REFERENCES "SwarmsInfo" ("Id") ON DELETE CASCADE
);

CREATE TABLE "UsersTeams" (
    "UserId" TEXT NOT NULL,
    "TeamId" TEXT NOT NULL,
    CONSTRAINT "PK_UsersTeams" PRIMARY KEY ("UserId", "TeamId"),
    CONSTRAINT "FK_UsersTeams_Teams_TeamId" FOREIGN KEY ("TeamId") REFERENCES "Teams" ("Id") ON DELETE CASCADE,
    CONSTRAINT "FK_UsersTeams_Users_UserId" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE
);

INSERT INTO "Roles" ("Id", "CreatedAt", "Name")
VALUES ('0196DEBD-033B-7512-A11B-98533D063A04', '2025-05-17 15:18:55.8133542', 'Admin');
SELECT changes();

INSERT INTO "Roles" ("Id", "CreatedAt", "Name")
VALUES ('0196DEBE-0C94-7467-9B2D-397E0200276F', '2025-05-17 15:18:55.8133889', 'Dev');
SELECT changes();

INSERT INTO "Roles" ("Id", "CreatedAt", "Name")
VALUES ('0196DEBE-2D80-76DD-B351-ADE38FA29169', '2025-05-17 15:18:55.8133891', 'QA');
SELECT changes();


INSERT INTO "Users" ("Id", "CreatedAt", "Email", "Name", "Password")
VALUES ('0196DED1-13F1-73FB-ACF0-188115C01C0E', '2025-05-17 15:18:55.8257228', 'dev@dev.com', 'dev', '7TO2PMxA2Z/tac0t3qCcRHXU+TNOYbVpXHYJj/oBoOGuADd5');
SELECT changes();

INSERT INTO "Users" ("Id", "CreatedAt", "Email", "Name", "Password")
VALUES ('0196DED1-13F1-743A-8A1B-5E243048C77E', '2025-05-17 15:18:55.8313565', 'qa@qa.com', 'qa', 'H8qXcmLJ6DNZJKIjfG/y4Qbzpd9B1qeDVFYClMXZK8dLSvAg');
SELECT changes();

INSERT INTO "Users" ("Id", "CreatedAt", "Email", "Name", "Password")
VALUES ('0196DED1-13F1-77CE-884E-3CB636EC09A8', '2025-05-17 15:18:55.8140825', 'admin@admin.com', 'admin', 'EM7te8rWhVPqXBuaXT5SjtrHLt+otJmPj0EnjviIf/jQSPwe');
SELECT changes();


INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A01-4B2D-8E1F-1A2B3C4D5E6F', 13, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A02-4B2D-8E1F-1A2B3C4D5E6F', 14, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A03-4B2D-8E1F-1A2B3C4D5E6F', 15, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A04-4B2D-8E1F-1A2B3C4D5E6F', 16, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A05-4B2D-8E1F-1A2B3C4D5E6F', 17, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A06-4B2D-8E1F-1A2B3C4D5E6F', 18, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A07-4B2D-8E1F-1A2B3C4D5E6F', 19, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A08-4B2D-8E1F-1A2B3C4D5E6F', 20, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A09-4B2D-8E1F-1A2B3C4D5E6F', 21, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0A-4B2D-8E1F-1A2B3C4D5E6F', 22, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0B-4B2D-8E1F-1A2B3C4D5E6F', 24, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0C-4B2D-8E1F-1A2B3C4D5E6F', 25, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0D-4B2D-8E1F-1A2B3C4D5E6F', 26, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0E-4B2D-8E1F-1A2B3C4D5E6F', 27, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A0F-4B2D-8E1F-1A2B3C4D5E6F', 28, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A10-4B2D-8E1F-1A2B3C4D5E6F', 30, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A11-4B2D-8E1F-1A2B3C4D5E6F', 29, '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A12-4B2D-8E1F-1A2B3C4D5E6F', 1, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A13-4B2D-8E1F-1A2B3C4D5E6F', 5, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A14-4B2D-8E1F-1A2B3C4D5E6F', 9, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A15-4B2D-8E1F-1A2B3C4D5E6F', 13, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A16-4B2D-8E1F-1A2B3C4D5E6F', 17, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A17-4B2D-8E1F-1A2B3C4D5E6F', 21, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A18-4B2D-8E1F-1A2B3C4D5E6F', 25, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Permissions" ("Id", "PermissionCode", "RoleId")
VALUES ('0196DEBE-3A19-4B2D-8E1F-1A2B3C4D5E6F', 28, '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();


INSERT INTO "Teams" ("Id", "Name", "RoleId")
VALUES ('0196DECE-E967-755F-9C6A-5BA5A34577F1', 'QA', '0196DEBE-2D80-76DD-B351-ADE38FA29169');
SELECT changes();

INSERT INTO "Teams" ("Id", "Name", "RoleId")
VALUES ('0196DECE-E967-7965-AFB9-F33FF2B3F0DC', 'Devs', '0196DEBE-0C94-7467-9B2D-397E0200276F');
SELECT changes();

INSERT INTO "Teams" ("Id", "Name", "RoleId")
VALUES ('0196DECE-E967-7D50-832C-0CA0155465A1', 'Admins', '0196DEBD-033B-7512-A11B-98533D063A04');
SELECT changes();


INSERT INTO "UsersTeams" ("TeamId", "UserId")
VALUES ('0196DECE-E967-7965-AFB9-F33FF2B3F0DC', '0196DED1-13F1-73FB-ACF0-188115C01C0E');
SELECT changes();

INSERT INTO "UsersTeams" ("TeamId", "UserId")
VALUES ('0196DECE-E967-755F-9C6A-5BA5A34577F1', '0196DED1-13F1-743A-8A1B-5E243048C77E');
SELECT changes();

INSERT INTO "UsersTeams" ("TeamId", "UserId")
VALUES ('0196DECE-E967-7D50-832C-0CA0155465A1', '0196DED1-13F1-77CE-884E-3CB636EC09A8');
SELECT changes();


CREATE INDEX "IX_ContainerStats_ContainerInfoId" ON "ContainerStats" ("ContainerInfoId");

CREATE UNIQUE INDEX "IX_ContainersInfo_ContainerId" ON "ContainersInfo" ("ContainerId");

CREATE INDEX "IX_ContainersInfo_PlatformId" ON "ContainersInfo" ("PlatformId");

CREATE INDEX "IX_Permissions_RoleId" ON "Permissions" ("RoleId");

CREATE UNIQUE INDEX "IX_PlatformStats_Created" ON "PlatformStats" ("Created");

CREATE INDEX "IX_PlatformStats_PlatformId" ON "PlatformStats" ("PlatformId");

CREATE UNIQUE INDEX "AddressIndex" ON "Platforms" ("Address");

CREATE INDEX "IX_RefreshTokens_UserId" ON "RefreshTokens" ("UserId");

CREATE UNIQUE INDEX "IX_Registries_Name" ON "Registries" ("Name");

CREATE UNIQUE INDEX "IX_SwarmsInfo_PlatformId" ON "SwarmsInfo" ("PlatformId");

CREATE INDEX "IX_SwarmsPeer_SwarmInfoId" ON "SwarmsPeer" ("SwarmInfoId");

CREATE INDEX "IX_Teams_RoleId" ON "Teams" ("RoleId");

CREATE UNIQUE INDEX "EmailIndex" ON "Users" ("Email");

CREATE INDEX "IX_UsersTeams_TeamId" ON "UsersTeams" ("TeamId");

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20250517151856_migration0001', '9.0.4');

COMMIT;

