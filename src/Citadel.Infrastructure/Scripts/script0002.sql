START TRANSACTION;
CREATE TABLE backuprepositories (
    id uuid NOT NULL,
    archivedat timestamp with time zone,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    description text,
    lastcheckedat timestamp with time zone,
    lastprunedat timestamp with time zone,
    name text NOT NULL,
    normalizedname text NOT NULL,
    passwordsecretid uuid NOT NULL,
    rowversion bigint NOT NULL DEFAULT 0,
    spec jsonb NOT NULL,
    status text NOT NULL,
    type text NOT NULL,
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_backuprepositories PRIMARY KEY (id),
    CONSTRAINT fk_backuprepositories_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuprepositories_secretdefinitions_passwordsecretid FOREIGN KEY (passwordsecretid) REFERENCES secretdefinitions (id) ON DELETE RESTRICT
);

CREATE TABLE backupsourceleases (
    sourcekey text NOT NULL,
    createdat timestamp with time zone NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    operationtype text NOT NULL,
    ownerrunid uuid NOT NULL,
    CONSTRAINT pk_backupsourceleases PRIMARY KEY (sourcekey)
);

CREATE TABLE backuppolicies (
    id uuid NOT NULL,
    alertonfailure boolean NOT NULL DEFAULT TRUE,
    archivedat timestamp with time zone,
    backuprepositoryid uuid NOT NULL,
    controlstate text NOT NULL DEFAULT 'Idle',
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    cron text,
    currentrunid uuid,
    description text,
    enabled boolean NOT NULL DEFAULT TRUE,
    firstsuccessfulrunat timestamp with time zone,
    keeplastsuccessful integer NOT NULL DEFAULT 14,
    lastscheduledrunat timestamp with time zone,
    name text NOT NULL,
    normalizedname text NOT NULL,
    rowversion bigint NOT NULL DEFAULT 0,
    runasactorid uuid NOT NULL,
    source jsonb NOT NULL,
    timezone text,
    timeoutseconds integer NOT NULL DEFAULT 14400,
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_backuppolicies PRIMARY KEY (id),
    CONSTRAINT fk_backuppolicies_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuppolicies_actors_runasactorid FOREIGN KEY (runasactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuppolicies_backuprepositories_backuprepositoryid FOREIGN KEY (backuprepositoryid) REFERENCES backuprepositories (id) ON DELETE RESTRICT
);

CREATE TABLE backuprepositoryleases (
    backuprepositoryid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    operationtype text NOT NULL,
    ownerrunid uuid NOT NULL,
    CONSTRAINT pk_backuprepositoryleases PRIMARY KEY (backuprepositoryid),
    CONSTRAINT fk_backuprepositoryleases_backuprepositories_backuprepositoryid FOREIGN KEY (backuprepositoryid) REFERENCES backuprepositories (id) ON DELETE CASCADE
);

CREATE TABLE backuprepositoryvalidations (
    id uuid NOT NULL,
    backuprepositoryid uuid NOT NULL,
    lasterrorcode text,
    lasterrormessage text,
    lastvalidatedat timestamp with time zone NOT NULL,
    location text NOT NULL,
    platformid uuid,
    status text NOT NULL,
    CONSTRAINT pk_backuprepositoryvalidations PRIMARY KEY (id),
    CONSTRAINT "fk_backuprepositoryvalidations_backuprepositories_backupreposi~" FOREIGN KEY (backuprepositoryid) REFERENCES backuprepositories (id) ON DELETE CASCADE,
    CONSTRAINT fk_backuprepositoryvalidations_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE SET NULL
);

CREATE TABLE backupruns (
    id uuid NOT NULL,
    backuppolicyid uuid NOT NULL,
    backuprepositoryid uuid NOT NULL,
    bytesadded bigint,
    bytesprocessed bigint,
    completedat timestamp with time zone,
    errorcode text,
    errormessage text,
    exitcode integer,
    filesprocessed bigint,
    parentsnapshotid text,
    policynamesnapshot text NOT NULL,
    queuedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    repositorytypesnapshot text NOT NULL,
    resticsnapshotid text,
    snapshotavailability text NOT NULL,
    sourcesnapshot jsonb NOT NULL,
    startedat timestamp with time zone,
    status text NOT NULL,
    trigger text NOT NULL,
    triggersourceid uuid,
    triggeredbyactorid uuid NOT NULL,
    warnings jsonb NOT NULL DEFAULT ('[]'::jsonb),
    CONSTRAINT pk_backupruns PRIMARY KEY (id),
    CONSTRAINT fk_backupruns_actors_triggeredbyactorid FOREIGN KEY (triggeredbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backupruns_backuppolicies_backuppolicyid FOREIGN KEY (backuppolicyid) REFERENCES backuppolicies (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backupruns_backuprepositories_backuprepositoryid FOREIGN KEY (backuprepositoryid) REFERENCES backuprepositories (id) ON DELETE RESTRICT
);

CREATE TABLE backuprestoreruns (
    id uuid NOT NULL,
    affectedcontainers jsonb NOT NULL DEFAULT ('[]'::jsonb),
    backuprepositoryid uuid NOT NULL,
    backuprunid uuid NOT NULL,
    completedat timestamp with time zone,
    errorcode text,
    errormessage text,
    exitcode integer,
    overwriteexisting boolean NOT NULL,
    queuedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    startedat timestamp with time zone,
    status text NOT NULL,
    targetplatformid uuid NOT NULL,
    targetvolumecreatedbycitadel boolean NOT NULL,
    targetvolumename text NOT NULL,
    triggeredbyactorid uuid NOT NULL,
    warnings jsonb NOT NULL DEFAULT ('[]'::jsonb),
    CONSTRAINT pk_backuprestoreruns PRIMARY KEY (id),
    CONSTRAINT fk_backuprestoreruns_actors_triggeredbyactorid FOREIGN KEY (triggeredbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuprestoreruns_backuprepositories_backuprepositoryid FOREIGN KEY (backuprepositoryid) REFERENCES backuprepositories (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuprestoreruns_backupruns_backuprunid FOREIGN KEY (backuprunid) REFERENCES backupruns (id) ON DELETE RESTRICT,
    CONSTRAINT fk_backuprestoreruns_platforms_targetplatformid FOREIGN KEY (targetplatformid) REFERENCES platforms (id) ON DELETE RESTRICT
);

CREATE TABLE backuprunlogs (
    id uuid NOT NULL,
    backuprunid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    message text NOT NULL,
    stream text NOT NULL,
    CONSTRAINT pk_backuprunlogs PRIMARY KEY (id),
    CONSTRAINT fk_backuprunlogs_backupruns_backuprunid FOREIGN KEY (backuprunid) REFERENCES backupruns (id) ON DELETE CASCADE
);

CREATE TABLE backuprestorerunlogs (
    id uuid NOT NULL,
    backuprestorerunid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL,
    message text NOT NULL,
    stream text NOT NULL,
    CONSTRAINT pk_backuprestorerunlogs PRIMARY KEY (id),
    CONSTRAINT fk_backuprestorerunlogs_backuprestoreruns_backuprestorerunid FOREIGN KEY (backuprestorerunid) REFERENCES backuprestoreruns (id) ON DELETE CASCADE
);

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('102eae03-0582-a53c-2287-ce55c2f222a8', 2, 16, '30000000-0000-0000-0000-000000000002', 128);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('378efbb8-bac7-1928-e93a-7d152a50b3b6', 1, 15, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('6128787e-901d-f15b-c094-054be267a6c8', 1, 16, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('7cb0a723-f754-2be2-38fa-8d151c2e1c7f', 2, 15, '30000000-0000-0000-0000-000000000002', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('ba1393e4-1090-990e-aee3-a3e36ede0fee', 4, 16, '30000000-0000-0000-0000-000000000001', 128);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('e9b46175-cc60-8d02-1dbd-ddf7f907eb16', 4, 15, '30000000-0000-0000-0000-000000000001', 0);

CREATE INDEX ix_backuppolicies_archivedat ON backuppolicies (archivedat);

CREATE INDEX ix_backuppolicies_backuprepositoryid ON backuppolicies (backuprepositoryid);

CREATE INDEX ix_backuppolicies_createdbyactorid ON backuppolicies (createdbyactorid);

CREATE UNIQUE INDEX ix_backuppolicies_normalizedname ON backuppolicies (normalizedname);

CREATE INDEX ix_backuppolicies_runasactorid ON backuppolicies (runasactorid);

CREATE INDEX ix_backuppolicies_schedule ON backuppolicies (enabled, cron);

CREATE INDEX ix_backuprepositories_archivedat ON backuprepositories (archivedat);

CREATE INDEX ix_backuprepositories_createdbyactorid ON backuprepositories (createdbyactorid);

CREATE UNIQUE INDEX ix_backuprepositories_normalizedname ON backuprepositories (normalizedname);

CREATE INDEX ix_backuprepositories_passwordsecretid ON backuprepositories (passwordsecretid);

CREATE INDEX ix_backuprepositories_status ON backuprepositories (status);

CREATE INDEX ix_backuprepositoryleases_expiresat ON backuprepositoryleases (expiresat);

CREATE INDEX ix_backuprepositoryvalidations_platformid ON backuprepositoryvalidations (platformid);

CREATE INDEX ix_backuprepositoryvalidations_repository_location_platform ON backuprepositoryvalidations (backuprepositoryid, location, platformid);

CREATE INDEX ix_backuprestorerunlogs_restorerun_createdat ON backuprestorerunlogs (backuprestorerunid, createdat);

CREATE INDEX ix_backuprestoreruns_backuprun_queuedat ON backuprestoreruns (backuprunid, queuedat);

CREATE INDEX ix_backuprestoreruns_queuedat ON backuprestoreruns (queuedat);

CREATE INDEX ix_backuprestoreruns_repository_status ON backuprestoreruns (backuprepositoryid, status);

CREATE INDEX ix_backuprestoreruns_status_queuedat ON backuprestoreruns (status, queuedat);

CREATE INDEX ix_backuprestoreruns_targetvolume ON backuprestoreruns (targetplatformid, targetvolumename);

CREATE INDEX ix_backuprestoreruns_triggeredbyactorid ON backuprestoreruns (triggeredbyactorid);

CREATE INDEX ix_backuprunlogs_run_createdat ON backuprunlogs (backuprunid, createdat);

CREATE UNIQUE INDEX ix_backupruns_active_policy ON backupruns (backuppolicyid) WHERE status IN ('Queued', 'Preparing', 'Running', 'ApplyingRetention');

CREATE INDEX ix_backupruns_policy_queuedat ON backupruns (backuppolicyid, queuedat);

CREATE INDEX ix_backupruns_queuedat ON backupruns (queuedat);

CREATE INDEX ix_backupruns_repository_status ON backupruns (backuprepositoryid, status);

CREATE INDEX ix_backupruns_snapshotavailability ON backupruns (snapshotavailability);

CREATE INDEX ix_backupruns_status_queuedat ON backupruns (status, queuedat);

CREATE INDEX ix_backupruns_triggeredbyactorid ON backupruns (triggeredbyactorid);

CREATE INDEX ix_backupsourceleases_expiresat ON backupsourceleases (expiresat);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260712195605_migration0002', '10.0.9');

COMMIT;

