-- Migration: 20250708071103_Schema.sql
-- Generated: 2025-07-08 07:11:04 UTC

CREATE TABLE IF NOT EXISTS "Platforms" ("Id" TEXT NOT NULL, "Name" TEXT NOT NULL, "Address" TEXT NOT NULL, "Status" TEXT NOT NULL, "ConnectorType" TEXT NOT NULL, "NetworkCount" INTEGER NOT NULL, "VolumeCount" INTEGER NOT NULL, "ImageCount" INTEGER NOT NULL, "CpuCount" INTEGER NOT NULL, "MemTotal" INTEGER NOT NULL, "AgentVersion" TEXT, "ServerVersion" TEXT, "PlatformDescriptor" TEXT NOT NULL, CONSTRAINT "PK_Platforms" PRIMARY KEY ("Id"))
;
CREATE UNIQUE INDEX IF NOT EXISTS "AddressIndex" ON "Platforms" ("Address" ASC)
;
CREATE TABLE IF NOT EXISTS "PlatformStats" ("Id" TEXT NOT NULL, "PlatformId" TEXT NOT NULL, "Created" INTEGER NOT NULL, "MemoryUsage" REAL NOT NULL, "CpuUsage" REAL NOT NULL, "RxBytes" REAL NOT NULL, "TxBytes" REAL NOT NULL, CONSTRAINT "PK_PlatformStats" PRIMARY KEY ("Id"), CONSTRAINT "FK_PlatformStats_Platforms" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_PlatformStats_PlatformId" ON "PlatformStats" ("PlatformId" ASC)
;
CREATE UNIQUE INDEX IF NOT EXISTS "IX_PlatformStats_Created" ON "PlatformStats" ("Created" ASC)
;
CREATE TABLE IF NOT EXISTS "Registries" ("Id" TEXT NOT NULL, "Name" TEXT NOT NULL, "Url" TEXT NOT NULL, "Created" TEXT NOT NULL, "Type" TEXT NOT NULL, "Configuration" TEXT NOT NULL, CONSTRAINT "PK_Registries" PRIMARY KEY ("Id"))
;
CREATE UNIQUE INDEX IF NOT EXISTS "IX_Registries_Name" ON "Registries" ("Name" ASC)
;
CREATE TABLE IF NOT EXISTS "Roles" ("Id" TEXT NOT NULL, "Name" TEXT NOT NULL, "CreatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00', "UpdatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00', CONSTRAINT "PK_Roles" PRIMARY KEY ("Id"))
;
CREATE TABLE IF NOT EXISTS "Users" ("Id" TEXT NOT NULL, "Name" TEXT NOT NULL, "Email" TEXT NOT NULL, "Password" TEXT NOT NULL, "CreatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00', "UpdatedAt" TEXT NOT NULL DEFAULT '2000-01-01 00:00:00', CONSTRAINT "PK_Users" PRIMARY KEY ("Id"))
;
CREATE UNIQUE INDEX IF NOT EXISTS "EmailIndex" ON "Users" ("Email" ASC)
;
CREATE TABLE IF NOT EXISTS "Permissions" ("Id" TEXT NOT NULL, "RoleId" TEXT NOT NULL, "PermissionCode" INTEGER NOT NULL, CONSTRAINT "PK_Permissions" PRIMARY KEY ("Id"), CONSTRAINT "FK_Permissions_RoleId_Roles_Id" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_Permissions_RoleId" ON "Permissions" ("RoleId" ASC)
;
CREATE TABLE IF NOT EXISTS "Teams" ("Id" TEXT NOT NULL, "RoleId" TEXT NOT NULL, "Name" TEXT NOT NULL, CONSTRAINT "PK_Teams" PRIMARY KEY ("Id"), CONSTRAINT "FK_Teams_RoleId_Roles_Id" FOREIGN KEY ("RoleId") REFERENCES "Roles" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_Teams_RoleId" ON "Teams" ("RoleId" ASC)
;
CREATE TABLE IF NOT EXISTS "RefreshTokens" ("Id" TEXT NOT NULL, "UserId" TEXT NOT NULL, "CreatedAt" TEXT NOT NULL, CONSTRAINT "PK_RefreshTokens" PRIMARY KEY ("Id"), CONSTRAINT "FK_RefreshTokens_UserId_Users_Id" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_RefreshTokens_UserId" ON "RefreshTokens" ("UserId" ASC)
;
CREATE TABLE IF NOT EXISTS "UsersTeams" ("UserId" TEXT NOT NULL, "TeamId" TEXT NOT NULL, CONSTRAINT "PK_UsersTeams" PRIMARY KEY ("UserId", "TeamId"), CONSTRAINT "FK_UsersTeams_UserId_Users_Id" FOREIGN KEY ("UserId") REFERENCES "Users" ("Id") ON DELETE CASCADE ON UPDATE CASCADE, CONSTRAINT "FK_UsersTeams_TeamId_Teams_Id" FOREIGN KEY ("TeamId") REFERENCES "Teams" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_UsersTeams_TeamId" ON "UsersTeams" ("TeamId" ASC)
;
CREATE INDEX IF NOT EXISTS "IX_UsersTeams_UserId" ON "UsersTeams" ("UserId" ASC)
;
CREATE TABLE IF NOT EXISTS "Containers" ("Id" TEXT NOT NULL, "PlatformId" TEXT NOT NULL, "ContainerId" TEXT NOT NULL, "Name" TEXT NOT NULL, "Image" TEXT NOT NULL, "Created" INTEGER NOT NULL, "Updated" INTEGER NOT NULL, "State" TEXT NOT NULL, "Stack" TEXT, "Ports" TEXT NOT NULL, CONSTRAINT "PK_Containers" PRIMARY KEY ("Id"), CONSTRAINT "FK_Containers_Platforms" FOREIGN KEY ("PlatformId") REFERENCES "Platforms" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE UNIQUE INDEX IF NOT EXISTS "IX_Containers_ContainerId" ON "Containers" ("ContainerId" ASC)
;
CREATE INDEX IF NOT EXISTS "IX_Containers_PlatformId" ON "Containers" ("PlatformId" ASC)
;
CREATE TABLE IF NOT EXISTS "ContainerStats" ("Id" TEXT NOT NULL, "ContainerId" TEXT NOT NULL, "Created" INTEGER NOT NULL, "MemoryUsage" REAL, "CpuUsage" REAL, "MemoryLimit" REAL, "RxBytes" REAL, "TxBytes" REAL, CONSTRAINT "PK_ContainerStats" PRIMARY KEY ("Id"), CONSTRAINT "FK_ContainerStats_ContainerId_Containers_Id" FOREIGN KEY ("ContainerId") REFERENCES "Containers" ("Id") ON DELETE CASCADE ON UPDATE CASCADE)
;
CREATE INDEX IF NOT EXISTS "IX_ContainerStats_ContainerId" ON "ContainerStats" ("ContainerId" ASC)
;
