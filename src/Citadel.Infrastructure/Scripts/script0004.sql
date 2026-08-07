START TRANSACTION;
ALTER TABLE swarmserviceprojections ADD forceupdate bigint NOT NULL DEFAULT 0;

ALTER TABLE swarmserviceprojections ADD liveruntimehash text;

ALTER TABLE swarmserviceprojections ADD swarmserviceid uuid;

CREATE TABLE swarmservices (
    id uuid NOT NULL,
    appliedimagedigest text,
    attemptedat timestamp with time zone,
    autoupdatestate_currentdigest text,
    autoupdatestate_lastcheckedat timestamp with time zone,
    autoupdatestate_lasterror text,
    autoupdatestate_remotedigest text,
    autoupdatestate_status text,
    basedockerversion bigint,
    completedat timestamp with time zone,
    controlstartedat bigint,
    controlstate text NOT NULL DEFAULT 'Idle',
    controltriggeredby uuid,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    createdbyactorid uuid NOT NULL,
    description text,
    desiredspechash text NOT NULL,
    dockername text NOT NULL,
    dockerserviceid text,
    dockerversionindex bigint,
    expectedforceupdate bigint,
    health text NOT NULL,
    lastapplieddesiredspechash text,
    lastappliedruntimehash text,
    name text NOT NULL,
    observeddockerversion bigint,
    operationactorid uuid,
    operationclusterid text,
    operationid uuid,
    operationkind text,
    operationstate text,
    platformid uuid NOT NULL,
    preparedat timestamp with time zone,
    resultcode text,
    resultmessage text,
    rowversion bigint NOT NULL DEFAULT 0,
    spec jsonb NOT NULL,
    synchronizationstate text NOT NULL,
    targetdesiredspechash text,
    targetrowversion bigint,
    targetruntimehash text,
    updatedat timestamp with time zone NOT NULL,
    warnings jsonb,
    CONSTRAINT pk_swarmservices PRIMARY KEY (id),
    CONSTRAINT "CK_SwarmServices_CanceledOperation" CHECK (operationstate <> 'Canceled' OR (attemptedat IS NULL AND completedat IS NOT NULL)),
    CONSTRAINT "CK_SwarmServices_OperationFields" CHECK ((operationid IS NULL AND operationkind IS NULL AND operationstate IS NULL AND basedockerversion IS NULL AND targetdesiredspechash IS NULL AND targetruntimehash IS NULL AND targetrowversion IS NULL AND expectedforceupdate IS NULL AND preparedat IS NULL AND attemptedat IS NULL AND completedat IS NULL AND observeddockerversion IS NULL AND resultcode IS NULL AND warnings IS NULL AND resultmessage IS NULL AND operationclusterid IS NULL AND operationactorid IS NULL) OR (operationid IS NOT NULL AND operationkind IS NOT NULL AND operationstate IS NOT NULL AND targetdesiredspechash IS NOT NULL AND targetrowversion IS NOT NULL AND preparedat IS NOT NULL AND operationclusterid IS NOT NULL AND operationactorid IS NOT NULL)),
    CONSTRAINT fk_swarmservices_actors_controltriggeredby FOREIGN KEY (controltriggeredby) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_swarmservices_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_swarmservices_platforms_platformid FOREIGN KEY (platformid) REFERENCES platforms (id) ON DELETE RESTRICT
);

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('00389706-8a88-9cfa-583a-1d4b7d2ce63a', 4, 20, '30000000-0000-0000-0000-000000000001', 39);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('0d39d935-1b2b-bf74-bfb3-51d40c4cfc56', 1, 20, '30000000-0000-0000-0000-000000000003', 0);
INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('f2c76082-b7aa-7bea-bb5f-d22de2632a62', 2, 20, '30000000-0000-0000-0000-000000000002', 39);

CREATE INDEX ix_swarmserviceprojections_swarmserviceid ON swarmserviceprojections (swarmserviceid);

CREATE INDEX ix_swarmservices_controltriggeredby ON swarmservices (controltriggeredby);

CREATE INDEX ix_swarmservices_createdbyactorid ON swarmservices (createdbyactorid);

CREATE UNIQUE INDEX ix_swarmservices_dockername_platformid ON swarmservices (dockername, platformid);

CREATE INDEX ix_swarmservices_globalsearch_name_trgm ON swarmservices USING gin (name gin_trgm_ops);

CREATE UNIQUE INDEX ix_swarmservices_name_platformid ON swarmservices (name, platformid);

CREATE UNIQUE INDEX ix_swarmservices_platformid_dockerserviceid ON swarmservices (platformid, dockerserviceid) WHERE "dockerserviceid" IS NOT NULL;

CREATE INDEX ix_swarmservices_recoverableoperation ON swarmservices (operationstate, preparedat);

INSERT INTO "__EFMigrationsHistory" ("MigrationId", "ProductVersion")
VALUES ('20260806101431_migration0004', '10.0.10');

COMMIT;
