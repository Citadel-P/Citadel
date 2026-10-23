-- Secret use is a durable grant, independent of knowledge of a UUID or a binding.
CREATE TABLE secretusegrants (
    secretid uuid NOT NULL REFERENCES secretdefinitions(id) ON DELETE CASCADE,
    scope text NOT NULL CHECK (scope IN ('Global','Creator','Stack','Deployment','SwarmService','Build','BackupRepository')),
    resourceid uuid NOT NULL,
    PRIMARY KEY(secretid, scope, resourceid)
);
INSERT INTO secretusegrants SELECT DISTINCT secretid, scope, COALESCE(resourceid, '00000000-0000-0000-0000-000000000000'::uuid)
FROM resourcebindings WHERE secretid IS NOT NULL ON CONFLICT DO NOTHING;
INSERT INTO secretusegrants SELECT passwordsecretid, 'BackupRepository', id FROM backuprepositories ON CONFLICT DO NOTHING;
INSERT INTO secretusegrants SELECT (spec->>field)::uuid, 'BackupRepository', id FROM backuprepositories,
unnest(ARRAY['accessKeySecretId','secretKeySecretId','sessionTokenSecretId']) field
WHERE spec->>field IS NOT NULL ON CONFLICT DO NOTHING;
INSERT INTO secretusegrants SELECT (secret->>'secretId')::uuid, 'Build', project.id
FROM buildprojects project, jsonb_array_elements(project.buildsecrets) secret
WHERE EXISTS (SELECT 1 FROM secretdefinitions WHERE id=(secret->>'secretId')::uuid) ON CONFLICT DO NOTHING;
-- External definitions are created by the global binding administrator.
INSERT INTO secretusegrants SELECT id, 'Global', '00000000-0000-0000-0000-000000000000'::uuid
FROM secretdefinitions WHERE providertype='VaultCompatibleKvV2' ON CONFLICT DO NOTHING;
