INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000015', 86400, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Git Update Available - Stack', '[]', NULL, 'Info', NULL, 'StackGitUpdateAvailable')
ON CONFLICT (id) DO NOTHING;

INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000016', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Git Stack Auto Updated', '[]', NULL, 'Info', NULL, 'StackGitAutoUpdated')
ON CONFLICT (id) DO NOTHING;

INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, threshold, type)
VALUES ('019d0000-0001-7000-8001-000000000017', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Git Auto Deploy Failed - Stack', '[]', NULL, 'Critical', NULL, 'StackGitAutoDeployFailed')
ON CONFLICT (id) DO NOTHING;
