BEGIN;

CREATE TABLE IF NOT EXISTS actions (
    id uuid NOT NULL,
    name text NOT NULL,
    description text,
    code text NOT NULL,
    defaultargsjson jsonb NOT NULL DEFAULT '{}'::jsonb,
    enabled boolean NOT NULL DEFAULT TRUE,
    scheduleenabled boolean NOT NULL DEFAULT FALSE,
    schedulecron text,
    scheduletimezone text NOT NULL DEFAULT 'UTC',
    webhook jsonb,
    timeoutseconds integer NOT NULL DEFAULT 300,
    alertonfailure boolean NOT NULL DEFAULT FALSE,
    runasactorid uuid NOT NULL,
    lastscheduledrunat timestamp with time zone,
    controlstate text NOT NULL DEFAULT 'Idle',
    currentrunid uuid,
    rowversion bigint NOT NULL DEFAULT 0,
    createdbyactorid uuid NOT NULL,
    createdat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updatedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    CONSTRAINT pk_actions PRIMARY KEY (id),
    CONSTRAINT fk_actions_actors_createdbyactorid FOREIGN KEY (createdbyactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_actions_actors_runasactorid FOREIGN KEY (runasactorid) REFERENCES actors (id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS actionruns (
    id uuid NOT NULL,
    actionid uuid NOT NULL,
    actionname text NOT NULL,
    trigger text NOT NULL,
    status text NOT NULL,
    runasactorid uuid NOT NULL,
    triggeredbyactorid uuid,
    argsjson jsonb NOT NULL DEFAULT '{}'::jsonb,
    codesnapshot text NOT NULL,
    codehash text NOT NULL,
    timeoutseconds integer NOT NULL,
    queuedat timestamp with time zone NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    startedat timestamp with time zone,
    finishedat timestamp with time zone,
    durationms bigint,
    exitcode integer,
    logs text,
    errormessage text,
    CONSTRAINT pk_actionruns PRIMARY KEY (id),
    CONSTRAINT fk_actionruns_actions_actionid FOREIGN KEY (actionid) REFERENCES actions (id) ON DELETE CASCADE,
    CONSTRAINT fk_actionruns_actors_runasactorid FOREIGN KEY (runasactorid) REFERENCES actors (id) ON DELETE RESTRICT,
    CONSTRAINT fk_actionruns_actors_triggeredbyactorid FOREIGN KEY (triggeredbyactorid) REFERENCES actors (id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ix_actions_name ON actions (name);
CREATE INDEX IF NOT EXISTS ix_actions_enabled ON actions (enabled);
CREATE INDEX IF NOT EXISTS ix_actions_schedule ON actions (enabled, scheduleenabled, schedulecron);
CREATE INDEX IF NOT EXISTS ix_actions_createdbyactorid ON actions (createdbyactorid);
CREATE INDEX IF NOT EXISTS ix_actions_runasactorid ON actions (runasactorid);

ALTER TABLE actions ADD COLUMN IF NOT EXISTS webhook jsonb;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_name = 'actions'
          AND column_name = 'webhookenabled'
    ) THEN
        UPDATE actions
        SET webhook = jsonb_build_object(
            'enabled', webhookenabled,
            'provider', 'GitHub',
            'authScheme', 'GitHubHmacSha256')
        WHERE webhook IS NULL;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS ix_actionruns_actionid_queuedat ON actionruns (actionid, queuedat DESC);
CREATE INDEX IF NOT EXISTS ix_actionruns_status_queuedat ON actionruns (status, queuedat);
CREATE INDEX IF NOT EXISTS ix_actionruns_runasactorid ON actionruns (runasactorid);
CREATE INDEX IF NOT EXISTS ix_actionruns_triggeredbyactorid ON actionruns (triggeredbyactorid);

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('40000000-0000-0000-0000-000000000013', 4, 13, '30000000-0000-0000-0000-000000000001', 0)
ON CONFLICT (roleid, resourcetype) DO NOTHING;

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('40000000-0000-0000-0000-000000000113', 2, 13, '30000000-0000-0000-0000-000000000002', 0)
ON CONFLICT (roleid, resourcetype) DO NOTHING;

INSERT INTO permissions (id, permissionlevel, resourcetype, roleid, specificpermissions)
VALUES ('40000000-0000-0000-0000-000000000213', 1, 13, '30000000-0000-0000-0000-000000000003', 0)
ON CONFLICT (roleid, resourcetype) DO NOTHING;

COMMIT;
