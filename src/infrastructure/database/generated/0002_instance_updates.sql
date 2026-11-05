-- Append-only upgrade from the published initial catalog.

-- Instance update state is independent of Activity retention.
CREATE TABLE instanceupdatechecks (
    id integer NOT NULL PRIMARY KEY CHECK (id = 1),
    lastattempt timestamp with time zone,
    lastsuccess timestamp with time zone,
    error text,
    available jsonb
);

CREATE TABLE instanceupdatediscoveries (
    instanceid uuid NOT NULL,
    version text NOT NULL,
    discoveredat timestamp with time zone NOT NULL,
    PRIMARY KEY (instanceid, version)
);

CREATE TABLE instanceupdateplans (
    id uuid NOT NULL PRIMARY KEY,
    actorid uuid NOT NULL,
    expiresat timestamp with time zone NOT NULL,
    plan jsonb NOT NULL
);

CREATE TABLE instanceupdates (
    id uuid NOT NULL PRIMARY KEY,
    instanceid uuid NOT NULL,
    actorid uuid NOT NULL,
    idempotencykey uuid NOT NULL,
    ownsinstallation boolean NOT NULL DEFAULT TRUE,
    revision bigint NOT NULL CHECK (revision >= 0),
    operation jsonb NOT NULL,
    completion jsonb,
    UNIQUE (instanceid, idempotencykey)
);
CREATE UNIQUE INDEX ix_instanceupdates_owner ON instanceupdates(instanceid) WHERE ownsinstallation;

CREATE TABLE instanceupdateeventreceipts (
    operationid uuid NOT NULL REFERENCES instanceupdates(id),
    eventkey text NOT NULL,
    PRIMARY KEY (operationid, eventkey)
);

ALTER TABLE instancesetupstates
    ADD COLUMN updatecheckintervalhours integer NOT NULL DEFAULT 6,
    ADD CONSTRAINT ck_instancesetupstates_updatecheckintervalhours
        CHECK (updatecheckintervalhours IN (6, 12, 24));

-- Observed through signed build-pool health checks, fenced by pool configuration.
ALTER TABLE buildagentpools ADD COLUMN agentversion text;

-- Central Agent settings and durable operations use the existing Activities journal.
CREATE TABLE agentupdatesettings (
    id integer PRIMARY KEY CHECK (id = 1),
    schedule jsonb NOT NULL DEFAULT '{"targets":[],"enabled":false,"cron":"0 3 * * *","timeZone":"UTC"}',
    release jsonb,
    checkerror text,
    observations jsonb NOT NULL DEFAULT '[]',
    lastscheduledminute timestamptz
);
INSERT INTO agentupdatesettings(id) VALUES (1);
CREATE TABLE agentupdateoperations (
    id uuid PRIMARY KEY,
    requestid uuid NOT NULL UNIQUE,
    actorid uuid NOT NULL REFERENCES actors(id),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    operation jsonb NOT NULL,
    finished boolean NOT NULL DEFAULT false,
    createdat timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX ix_agentupdateoperations_active ON agentupdateoperations((true)) WHERE NOT finished;

-- Keep existing Agent membership and require explicit opt-in for scheduled Core updates.
UPDATE agentupdatesettings SET schedule = '{"includeCore":false}'::jsonb || schedule;
ALTER TABLE agentupdatesettings ALTER COLUMN schedule SET DEFAULT '{"includeCore":false,"targets":[],"enabled":false,"cron":"0 3 * * *","timeZone":"UTC"}';
CREATE TABLE updatescheduleruns (
    id uuid PRIMARY KEY,
    minute timestamptz NOT NULL UNIQUE,
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    run jsonb NOT NULL,
    finished boolean NOT NULL DEFAULT false
);
CREATE UNIQUE INDEX ix_updatescheduleruns_active ON updatescheduleruns((true)) WHERE NOT finished;

ALTER TABLE instanceupdatechecks ADD COLUMN availability jsonb;
ALTER TABLE agentupdatesettings ADD COLUMN availability jsonb;

INSERT INTO alertrules(id,name,type,severity,status,createdbyactorid,limitedto,quiethours,createdat)
VALUES
('019d0000-0001-7000-8001-000000000025', 'Core Update Available', 'CoreUpdateAvailable', 'Info', 'Enabled', '00000000-0000-0000-0000-000000000001', '[]', '[]', TIMESTAMPTZ '2026-01-01T00:00:00Z'),
('019d0000-0001-7000-8001-000000000026', 'Agent Updates Available', 'AgentUpdatesAvailable', 'Info', 'Enabled', '00000000-0000-0000-0000-000000000001', '[]', '[]', TIMESTAMPTZ '2026-01-01T00:00:00Z');

-- Optional Docker block-device counters; historical samples remain unknown.
ALTER TABLE containerstats ADD COLUMN blockreadbytes double precision, ADD COLUMN blockwritebytes double precision;
ALTER TABLE workloadstats ADD COLUMN blockreadbytes double precision, ADD COLUMN blockwritebytes double precision;
ALTER TABLE swarmservicestats ADD COLUMN blockreadbytes double precision, ADD COLUMN blockwritebytes double precision;
