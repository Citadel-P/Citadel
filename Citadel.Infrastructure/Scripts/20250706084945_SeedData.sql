-- Migration: 20250706084945_SeedData.sql
-- Generated: 2025-07-06 08:49:46 UTC

INSERT OR IGNORE INTO "Roles" ("Id", "Name", "CreatedAt", "UpdatedAt") VALUES ('bdde9601-3b03-1275-a11b-98533d063a04', 'Admin', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Roles" ("Id", "Name", "CreatedAt", "UpdatedAt") VALUES ('bede9601-940c-6774-9b2d-397e0200276f', 'Dev', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Roles" ("Id", "Name", "CreatedAt", "UpdatedAt") VALUES ('bede9601-802d-dd76-b351-ade38fa29169', 'QA', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Users" ("Id", "Name", "Email", "Password", "CreatedAt", "UpdatedAt") VALUES ('d1de9601-f113-ce77-884e-3cb636ec09a8', 'admin', 'admin@admin.com', 'o6hWzZ+DIuSZoHNjf5D1t6101vfm4w2kmPRiAZ3Xq53JMMl1', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Users" ("Id", "Name", "Email", "Password", "CreatedAt", "UpdatedAt") VALUES ('d1de9601-f113-fb73-acf0-188115c01c0e', 'dev', 'dev@dev.com', 'eBnEWmOx4+wNHGR/Tunt+Sz5y7y3CQxufbe3lO1vOKwFCrft', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Users" ("Id", "Name", "Email", "Password", "CreatedAt", "UpdatedAt") VALUES ('d1de9601-f113-3a74-8a1b-5e243048c77e', 'qa', 'qa@qa.com', 'MmjMzZZgclu4JrBkm1SbuTKP52DJncsGDuj+/NJe1VW3alHk', '2025-01-01 00:00:00', '2025-01-01 00:00:00')
;
INSERT OR IGNORE INTO "Teams" ("Id", "Name", "RoleId") VALUES ('cede9601-67e9-507d-832c-0ca0155465a1', 'Admins', 'bdde9601-3b03-1275-a11b-98533d063a04')
;
INSERT OR IGNORE INTO "Teams" ("Id", "Name", "RoleId") VALUES ('cede9601-67e9-6579-afb9-f33ff2b3f0dc', 'Devs', 'bede9601-940c-6774-9b2d-397e0200276f')
;
INSERT OR IGNORE INTO "Teams" ("Id", "Name", "RoleId") VALUES ('cede9601-67e9-5f75-9c6a-5ba5a34577f1', 'QA', 'bede9601-802d-dd76-b351-ade38fa29169')
;
INSERT OR IGNORE INTO "UsersTeams" ("UserId", "TeamId") VALUES ('d1de9601-f113-ce77-884e-3cb636ec09a8', 'cede9601-67e9-507d-832c-0ca0155465a1')
;
INSERT OR IGNORE INTO "UsersTeams" ("UserId", "TeamId") VALUES ('d1de9601-f113-fb73-acf0-188115c01c0e', 'cede9601-67e9-6579-afb9-f33ff2b3f0dc')
;
INSERT OR IGNORE INTO "UsersTeams" ("UserId", "TeamId") VALUES ('d1de9601-f113-3a74-8a1b-5e243048c77e', 'cede9601-67e9-5f75-9c6a-5ba5a34577f1')
;
INSERT OR IGNORE INTO "Permissions" ("Id", "RoleId", "PermissionCode") VALUES ('bede9601-0d3a-2d4b-8e1f-1a2b3c4d5e6f', 'bede9601-940c-6774-9b2d-397e0200276f', 26)
;
INSERT OR IGNORE INTO "Permissions" ("Id", "RoleId", "PermissionCode") VALUES ('bede9601-153a-2d4b-8e1f-1a2b3c4d5e6f', 'bede9601-802d-dd76-b351-ade38fa29169', 13)
;
