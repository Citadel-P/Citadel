INSERT INTO alertrules (id, cooldownseconds, createdat, createdbyactorid, description, limitedto, name, quiethours, requiredmatches, severity, status, threshold, type)
VALUES ('019d0000-0001-7000-8001-00000000001b', NULL, TIMESTAMPTZ '2026-01-01T00:00:00Z', '00000000-0000-0000-0000-000000000001', NULL, '[]', 'Build Run Failed', '[]', NULL, 'Critical', 'Enabled', NULL, 'BuildRunFailed')
ON CONFLICT (id) DO NOTHING;
