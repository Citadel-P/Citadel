START TRANSACTION;
CREATE TABLE buildprojects (
    id uuid NOT NULL,
    archivedat timestamp with time zone,
    branch text NOT NULL,
    buildargs jsonb NOT NULL DEFAULT ('[]'::jsonb),
    buildsecrets jsonb NOT NULL DEFAULT ('[]'::jsonb),
    contextpath text NOT NULL DEFAULT '.',
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    currentrunid uuid,
    description text,
    dockerfilepath text NOT NULL DEFAULT 'Dockerfile',
    enabled boolean NOT NULL DEFAULT TRUE,
    gitrepositoryid uuid NOT NULL,
    imagerepository text NOT NULL,
    name text NOT NULL,
    normalizedname text NOT NULL,
    platformid uuid NOT NULL,
    registryid uuid NOT NULL,
    retentionruncount integer NOT NULL DEFAULT 20,
    rowversion bigint NOT NULL DEFAULT 0,
    tagtemplates jsonb NOT NULL DEFAULT ('["{branch}-{shortSha}"]'::jsonb),
    target text,
    timeoutseconds integer NOT NULL DEFAULT 1800,
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_buildprojects PRIMARY KEY (id),
    CONSTRAINT fk_buildprojects_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_buildprojects_gitrepositories_gitrepositoryid FOREIGN KEY (gitrepositoryid) REFERENCES gitrepositories (id) ON DELETE RESTRICT,
    CONSTRAINT fk_buildprojects_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT,
    CONSTRAINT fk_buildprojects_registries_registryid FOREIGN KEY (registryid) REFERENCES registries (id) ON DELETE RESTRICT
);

CREATE TABLE buildruns (
    id uuid NOT NULL,
    branch text NOT NULL,
    buildargssnapshot jsonb NOT NULL DEFAULT ('[]'::jsonb),
    buildprojectid uuid NOT NULL,
    buildsecretidssnapshot jsonb NOT NULL DEFAULT ('[]'::jsonb),
    completedat timestamp with time zone,
    contextpath text NOT NULL,
    dockerfilepath text NOT NULL,
    errorcode text,
    errormessage text,
    exitcode integer,
    gitrepositoryid uuid NOT NULL,
    gitrepositorynamesnapshot text NOT NULL,
    imagedigest text,
    imagereferences jsonb NOT NULL DEFAULT ('[]'::jsonb),
    imagerepository text NOT NULL,
    platformsnapshot jsonb NOT NULL,
    projectnamesnapshot text NOT NULL,
    queuedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    registrysnapshot jsonb NOT NULL,
    resolvedcommitsha text,
    startedat timestamp with time zone,
    status text NOT NULL,
    tagtemplatessnapshot jsonb NOT NULL DEFAULT ('[]'::jsonb),
    target text,
    timeoutseconds integer NOT NULL,
    trigger text NOT NULL,
    triggersourceid uuid,
    triggeredbyactorid uuid NOT NULL,
    CONSTRAINT pk_buildruns PRIMARY KEY (id),
    CONSTRAINT fk_buildruns_actors_triggeredbyactorid FOREIGN KEY (triggeredbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_buildruns_buildprojects_buildprojectid FOREIGN KEY (buildprojectid) REFERENCES buildprojects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_buildruns_gitrepositories_gitrepositoryid FOREIGN KEY (gitrepositoryid) REFERENCES gitrepositories (id) ON DELETE RESTRICT
);

CREATE TABLE buildrunlogs (
    id uuid NOT NULL,
    buildrunid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    message text NOT NULL,
    stream text NOT NULL,
    CONSTRAINT pk_buildrunlogs PRIMARY KEY (id),
    CONSTRAINT fk_buildrunlogs_buildruns_buildrunid FOREIGN KEY (buildrunid) REFERENCES buildruns (id) ON DELETE CASCADE
);

UPDATE actions SET code = 'const platformsResponse = await citadel.platforms.listPlatforms();
const platforms = platformsResponse?.platforms ?? [];
let pruned = 0;
let reclaimedBytes = 0;

for (const platform of platforms) {
  if (platform.status === ''Offline'') continue
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

console.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);'
WHERE id = '41000000-0000-0000-0000-000000000001';

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('6a2b1742-029b-d0df-1d2a-1d0aa1ed9a3f', 1, 18, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('c472d905-c03c-a9a8-0527-c2b540274078', 2, 18, '30000000-0000-0000-0000-000000000002', 4);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('d1af8dbf-ef7d-d81d-33f9-e3be5ee72243', 4, 18, '30000000-0000-0000-0000-000000000001', 4);

CREATE INDEX ix_buildprojects_archivedat ON buildprojects (archivedat);

CREATE INDEX ix_buildprojects_createdbyactorid ON buildprojects (createdbyactorid);

CREATE INDEX ix_buildprojects_gitrepositoryid ON buildprojects (gitrepositoryid);

CREATE UNIQUE INDEX ix_buildprojects_normalizedname ON buildprojects (normalizedname);

CREATE INDEX ix_buildprojects_platformid ON buildprojects (platformid);

CREATE INDEX ix_buildprojects_registryid ON buildprojects (registryid);

CREATE INDEX ix_buildrunlogs_run_createdat ON buildrunlogs (buildrunid, createdat);

CREATE UNIQUE INDEX ix_buildruns_active_project ON buildruns (buildprojectid) WHERE status IN ('Queued', 'Preparing', 'Running');

CREATE INDEX ix_buildruns_gitrepositoryid ON buildruns (gitrepositoryid);

CREATE INDEX ix_buildruns_project_queuedat ON buildruns (buildprojectid, queuedat);

CREATE INDEX ix_buildruns_queuedat ON buildruns (queuedat);

CREATE INDEX ix_buildruns_status_queuedat ON buildruns (status, queuedat);

CREATE INDEX ix_buildruns_triggeredbyactorid ON buildruns (triggeredbyactorid);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260718161355_migration0002_builds', '10.0.10');

COMMIT;

