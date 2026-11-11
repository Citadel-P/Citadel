-- Match the global Activities order and bounded retention predicates.
CREATE INDEX ix_activityevents_createdat_id ON activityevents(createdat DESC, id DESC);
CREATE INDEX ix_scriptruns_retention ON scriptruns(finishedat)
    WHERE status NOT IN ('Queued','Running');
CREATE INDEX ix_buildruns_retention ON buildruns(completedat)
    WHERE status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted');
CREATE INDEX ix_resourcecopies_review_expiry ON resourcecopies(createdat)
    WHERE status='Review';

-- Sample identity is already unique by source and capture time. These internal
-- UUIDs have no readers or foreign-key dependants; keep all observation values.
-- Reuse standalone unique indexes instead of building replacement history indexes.
ALTER TABLE containerstats DROP COLUMN id;
ALTER TABLE containerstats ADD CONSTRAINT pk_containerstats
    PRIMARY KEY USING INDEX ix_containerstats_containerid_created;
ALTER TABLE swarmservicestats DROP COLUMN id;
ALTER TABLE swarmservicestats ADD CONSTRAINT pk_swarmservicestats
    PRIMARY KEY USING INDEX ix_swarmservicestats_platformtaskcreated;

-- Constraint-owned unique indexes cannot be adopted as primary keys. Rebuild
-- only these two indexes in the same migration transaction, preserving uniqueness.
ALTER TABLE workloadstats DROP COLUMN id, DROP CONSTRAINT uq_workloadstats_container_created;
ALTER TABLE workloadstats ADD CONSTRAINT workloadstats_pkey PRIMARY KEY(containerid, created);
ALTER TABLE platformnodeiostats DROP COLUMN id,
    DROP CONSTRAINT platformnodeiostats_platformid_nodeid_created_key;
ALTER TABLE platformnodeiostats ADD CONSTRAINT platformnodeiostats_pkey
    PRIMARY KEY(platformid, nodeid, created);

-- Recycle expired tuples before the default 20% dead-row threshold. Keep the
-- server's vacuum cost limits and the existing seven-day retention policy.
ALTER TABLE containerstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE workloadstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE swarmservicestats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE platformstats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE platformnodeiostats SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);
ALTER TABLE containerhealthtransitions SET (autovacuum_vacuum_scale_factor=0.05, autovacuum_vacuum_threshold=1000);

-- Share measurements across container and workload/task history. Source and owner
-- identities outlive inventory; ownership belongs to the capture, not current state.
-- Fence old-version writers throughout backfill, before replacing their tables.
LOCK TABLE containerstats,workloadstats,swarmservicestats IN ACCESS EXCLUSIVE MODE;
CREATE TABLE statisticssources (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    platformid uuid NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    containerid uuid UNIQUE,
    dockertaskid text,
    compacteduntil bigint NOT NULL DEFAULT 0,
    UNIQUE(platformid, dockertaskid),
    CHECK (containerid IS NOT NULL OR dockertaskid IS NOT NULL)
);
CREATE TABLE statisticsowners (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    sourceid bigint NOT NULL REFERENCES statisticssources(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK(kind IN ('Workload','Service')),
    deploymentid uuid REFERENCES deployments(id) ON DELETE CASCADE,
    stackid uuid REFERENCES stacks(id) ON DELETE CASCADE,
    -- Discovered Swarm ownership is a historical snapshot, not a live FK (as in
    -- swarmservicestats). Deleted Stack identities must survive its migration.
    servicestackid uuid,
    swarmserviceid uuid,
    dockerserviceid text,
    dockertaskid text,
    servicename text NOT NULL,
    replicakey text NOT NULL,
    UNIQUE NULLS NOT DISTINCT(sourceid,kind,deploymentid,stackid,servicestackid,swarmserviceid,dockerserviceid,dockertaskid,servicename,replicakey)
);
CREATE INDEX ix_statisticsowners_deployment ON statisticsowners(deploymentid) WHERE deploymentid IS NOT NULL;
CREATE INDEX ix_statisticsowners_stack ON statisticsowners(stackid,servicename) WHERE stackid IS NOT NULL;
CREATE INDEX ix_statisticsowners_servicestack ON statisticsowners(servicestackid,servicename) WHERE servicestackid IS NOT NULL;
CREATE INDEX ix_statisticsowners_service ON statisticsowners(swarmserviceid) WHERE swarmserviceid IS NOT NULL;
CREATE INDEX ix_statisticssources_platform ON statisticssources(platformid);

CREATE TABLE statisticssamples (
    sourceid bigint NOT NULL REFERENCES statisticssources(id) ON DELETE CASCADE,
    ownerid bigint REFERENCES statisticsowners(id) ON DELETE SET NULL,
    created bigint NOT NULL,
    cpuusage double precision NOT NULL,
    memoryactive double precision NOT NULL,
    memorycache double precision NOT NULL,
    memorylimit double precision NOT NULL,
    rxbytes double precision NOT NULL,
    txbytes double precision NOT NULL,
    blockreadbytes double precision,
    blockwritebytes double precision,
    PRIMARY KEY(sourceid,created)
) PARTITION BY RANGE(created);
CREATE INDEX ix_statisticssamples_created ON statisticssamples(created);
CREATE INDEX ix_statisticssamples_owner_created ON statisticssamples(ownerid,created) WHERE ownerid IS NOT NULL;
CREATE TABLE statisticssamples_default PARTITION OF statisticssamples DEFAULT WITH (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000);

-- A rollup stores weighted averages and exact internal counter deltas, first/last
-- observations, elapsed durations and extrema. Raw observations remain separate.
CREATE TABLE statisticsrollups (
    sourceid bigint NOT NULL REFERENCES statisticssources(id) ON DELETE CASCADE,
    ownerid bigint REFERENCES statisticsowners(id) ON DELETE SET NULL,
    created bigint NOT NULL,
    cpuusage double precision NOT NULL,
    memoryactive double precision NOT NULL,
    memorycache double precision NOT NULL,
    memorylimit double precision NOT NULL,
    rxbytes double precision NOT NULL,
    txbytes double precision NOT NULL,
    blockreadbytes double precision,
    blockwritebytes double precision,
    samplecount integer NOT NULL CHECK(samplecount>0),
    readcount integer NOT NULL,
    writecount integer NOT NULL,
    lastcreated bigint NOT NULL,
    firstcounters double precision[] NOT NULL,
    lastcounters double precision[] NOT NULL,
    counterdeltas double precision[] NOT NULL,
    counterdurations bigint[] NOT NULL,
    peaks double precision[] NOT NULL,
    PRIMARY KEY(sourceid,created)
) PARTITION BY RANGE(created);
CREATE INDEX ix_statisticsrollups_created ON statisticsrollups(created);
CREATE INDEX ix_statisticsrollups_owner_created ON statisticsrollups(ownerid,created) WHERE ownerid IS NOT NULL;
CREATE TABLE statisticsrollups_default PARTITION OF statisticsrollups DEFAULT WITH (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000);

-- Callers serialize capture/compaction on the existing Platform row lock.
CREATE FUNCTION statistics_source(p uuid, c uuid, t text) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE existing_source statisticssources; result bigint;
BEGIN
    SELECT * INTO existing_source FROM statisticssources
    WHERE platformid=p AND ((c IS NOT NULL AND containerid=c) OR (t IS NOT NULL AND dockertaskid=t))
    ORDER BY (dockertaskid IS NOT DISTINCT FROM t) DESC LIMIT 1;
    IF FOUND THEN
        -- Inventory can expire while an offline Task still exists. A rediscovered
        -- projection may attach to that same physical Task after the old projection
        -- is gone; it must never steal a live container's source identity.
        IF c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid
            AND t IS NOT NULL AND t=existing_source.dockertaskid
            AND NOT EXISTS(SELECT 1 FROM containers WHERE id=existing_source.containerid) THEN
            UPDATE statisticssources SET containerid=c WHERE id=existing_source.id;
            existing_source.containerid := c;
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid)
            THEN
            RAISE EXCEPTION 'Statistics physical identity changed';
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NULL) OR (t IS NOT NULL AND t IS DISTINCT FROM existing_source.dockertaskid) THEN
            UPDATE statisticssources SET containerid=COALESCE(containerid,c),dockertaskid=COALESCE(t,dockertaskid) WHERE id=existing_source.id;
        END IF;
        RETURN existing_source.id;
    END IF;
    INSERT INTO statisticssources(platformid,containerid,dockertaskid) VALUES(p,c,t) RETURNING id INTO result;
    RETURN result;
END;
$$;
CREATE FUNCTION statistics_owner(s bigint,k text,d uuid,st uuid,sv uuid,ds text,n text,r text,task text DEFAULT NULL)
RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE result bigint; captured_task text;
BEGIN
    IF k='Service' THEN
        SELECT COALESCE(task,dockertaskid) INTO captured_task FROM statisticssources WHERE id=s;
    END IF;
    SELECT id INTO result FROM statisticsowners WHERE sourceid=s AND kind=k
        AND (deploymentid,stackid,servicestackid,swarmserviceid,dockerserviceid,dockertaskid,servicename,replicakey)
        IS NOT DISTINCT FROM (d,CASE WHEN k='Workload' THEN st END,CASE WHEN k='Service' THEN st END,sv,ds,captured_task,n,r);
    IF FOUND THEN RETURN result; END IF;
    INSERT INTO statisticsowners(sourceid,kind,deploymentid,stackid,servicestackid,swarmserviceid,dockerserviceid,dockertaskid,servicename,replicakey)
    VALUES(s,k,d,CASE WHEN k='Workload' THEN st END,CASE WHEN k='Service' THEN st END,sv,ds,captured_task,n,r) RETURNING id INTO result;
    RETURN result;
END;
$$;

-- Daily partitions are precreated. The default partition is a safe ingestion
-- fallback if maintenance was offline; it receives bounded ordinary retention.
CREATE FUNCTION statistics_partition(parent_name text, day_number bigint) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE child_name text; occupied boolean;
BEGIN
    IF parent_name NOT IN ('statisticssamples','statisticsrollups') OR day_number<0 THEN
        RAISE EXCEPTION 'Invalid statistics partition';
    END IF;
    child_name := parent_name || '_d' || day_number;
    IF to_regclass('public.' || child_name) IS NOT NULL THEN
        IF NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=to_regclass('public.' || child_name)
            AND inhparent=to_regclass('public.' || parent_name)) THEN
            RAISE EXCEPTION 'Statistics partition name belongs to another relation';
        END IF;
        RETURN false;
    END IF;
    EXECUTE format('SELECT EXISTS(SELECT 1 FROM %I WHERE created >= $1 AND created < $2)',parent_name || '_default')
        INTO occupied USING day_number*86400,(day_number+1)*86400;
    -- Never detach a populated default partition or hide its observations.
    IF occupied THEN RETURN false; END IF;
    EXECUTE format('CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%s) TO (%s)',child_name,parent_name,day_number*86400,(day_number+1)*86400);
    EXECUTE format('ALTER TABLE %I SET (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000)',child_name);
    RETURN true;
END;
$$;
DO $$ DECLARE d bigint; today bigint := floor(extract(epoch FROM now())/86400)::bigint;
BEGIN
    FOR d IN today-8..today+2 LOOP
        PERFORM statistics_partition('statisticssamples',d);
        PERFORM statistics_partition('statisticsrollups',d);
    END LOOP;
END $$;

-- Populate source identities before copying metrics. Deleted containers still
-- have workload capture identity, so their measurements are preserved too.
INSERT INTO statisticssources(platformid,containerid)
SELECT DISTINCT platformid,containerid FROM (
    SELECT c.platformid,s.containerid FROM containerstats s JOIN containers c ON c.id=s.containerid
    UNION SELECT platformid,containerid FROM workloadstats
) sources;
SELECT statistics_source(s.platformid,c.id,s.dockertaskid)
FROM (SELECT DISTINCT platformid,dockertaskid FROM swarmservicestats) s
LEFT JOIN swarmtaskprojections t ON t.platformid=s.platformid AND t.dockertaskid=s.dockertaskid
JOIN platforms p ON p.id=s.platformid
LEFT JOIN containers c ON c.platformid=t.platformid AND c.dockercontainerid=t.dockercontainerid
 AND COALESCE(c.dockernodeid,p.platformdescriptor::jsonb->>'nodeID')=t.dockernodeid;
INSERT INTO statisticsowners(sourceid,kind,deploymentid,stackid,servicename,replicakey)
SELECT DISTINCT src.id,'Workload',s.deploymentid,s.stackid,s.servicename,s.replicakey
FROM workloadstats s JOIN statisticssources src ON src.containerid=s.containerid;
INSERT INTO statisticsowners(sourceid,kind,servicestackid,swarmserviceid,dockerserviceid,dockertaskid,servicename,replicakey)
SELECT DISTINCT src.id,'Service',s.stackid,s.swarmserviceid,s.dockerserviceid,s.dockertaskid,s.servicename,s.taskkey
FROM swarmservicestats s JOIN statisticssources src ON src.platformid=s.platformid AND src.dockertaskid=s.dockertaskid;
-- New dictionaries/partitions have no planner statistics yet. Analyze before
-- validating whole-history equality; an EXISTS fast-start plan can otherwise
-- repeatedly scan history when the correct result is that no conflicts exist.
ANALYZE statisticssources;
ANALYZE statisticsowners;
INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
SELECT src.id,o.id,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes
FROM containerstats s JOIN statisticssources src ON src.containerid=s.containerid
LEFT JOIN workloadstats w ON w.containerid=s.containerid AND w.created=s.created
LEFT JOIN statisticsowners o ON o.sourceid=src.id AND o.kind='Workload'
 AND (o.deploymentid,o.stackid,o.servicename,o.replicakey) IS NOT DISTINCT FROM (w.deploymentid,w.stackid,w.servicename,w.replicakey);
ANALYZE statisticssamples;
DO $$ BEGIN
    IF (SELECT count(*) FROM workloadstats s JOIN statisticssources src ON src.containerid=s.containerid
        JOIN statisticssamples raw ON raw.sourceid=src.id AND raw.created=s.created
        WHERE (raw.cpuusage,raw.memoryactive,raw.memorycache,raw.memorylimit,raw.rxbytes,raw.txbytes,raw.blockreadbytes,raw.blockwritebytes) IS DISTINCT FROM (s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes))>0 THEN
        RAISE EXCEPTION 'Conflicting workloadstats measurements; original history remains unchanged';
    END IF;
END $$;
INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
SELECT src.id,o.id,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes FROM workloadstats s
JOIN statisticssources src ON src.containerid=s.containerid
JOIN statisticsowners o ON o.sourceid=src.id AND o.kind='Workload' AND (o.deploymentid,o.stackid,o.servicename,o.replicakey) IS NOT DISTINCT FROM (s.deploymentid,s.stackid,s.servicename,s.replicakey)
ON CONFLICT(sourceid,created) DO UPDATE SET ownerid=EXCLUDED.ownerid
WHERE statisticssamples.ownerid IS DISTINCT FROM EXCLUDED.ownerid;
DO $$ BEGIN
    IF (SELECT count(*) FROM swarmservicestats s JOIN statisticssources src ON src.platformid=s.platformid AND src.dockertaskid=s.dockertaskid
        JOIN statisticssamples raw ON raw.sourceid=src.id AND raw.created=s.created
        WHERE raw.ownerid IS NOT NULL OR (raw.cpuusage,raw.memoryactive,raw.memorycache,raw.memorylimit,raw.rxbytes,raw.txbytes,raw.blockreadbytes,raw.blockwritebytes) IS DISTINCT FROM (s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes))>0 THEN
        RAISE EXCEPTION 'Conflicting swarmservicestats measurements; original history remains unchanged';
    END IF;
END $$;
INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
SELECT src.id,o.id,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes FROM swarmservicestats s
JOIN statisticssources src ON src.platformid=s.platformid AND src.dockertaskid=s.dockertaskid
JOIN statisticsowners o ON o.sourceid=src.id AND o.kind='Service' AND (o.servicestackid,o.swarmserviceid,o.dockerserviceid,o.dockertaskid,o.servicename,o.replicakey) IS NOT DISTINCT FROM (s.stackid,s.swarmserviceid,s.dockerserviceid,s.dockertaskid,s.servicename,s.taskkey)
ON CONFLICT(sourceid,created) DO UPDATE SET ownerid=EXCLUDED.ownerid
WHERE statisticssamples.ownerid IS DISTINCT FROM EXCLUDED.ownerid;
DROP TABLE workloadstats;
DROP TABLE containerstats;
DROP TABLE swarmservicestats;

CREATE VIEW statisticsobservations AS
SELECT s.*,1::integer AS samplecount,
    (blockreadbytes IS NOT NULL)::integer AS readcount,
    (blockwritebytes IS NOT NULL)::integer AS writecount,
    created AS lastcreated,
    ARRAY[rxbytes,txbytes,blockreadbytes,blockwritebytes] AS firstcounters,
    ARRAY[rxbytes,txbytes,blockreadbytes,blockwritebytes] AS lastcounters,
    ARRAY[NULL,NULL,NULL,NULL]::double precision[] AS counterdeltas,
    ARRAY[0,0,0,0]::bigint[] AS counterdurations,
    ARRAY[cpuusage,memoryactive,memorycache,memorylimit] AS peaks FROM statisticssamples s
UNION ALL SELECT * FROM statisticsrollups;
CREATE VIEW containerstats AS
SELECT src.containerid,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN containers c ON c.id=src.containerid;
CREATE VIEW workloadstats AS
SELECT src.platformid,o.deploymentid,o.stackid,o.servicename,o.replicakey,src.containerid,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN statisticsowners o ON o.id=s.ownerid AND o.kind='Workload';
CREATE VIEW swarmservicestats AS
SELECT src.platformid,o.dockerserviceid,o.dockertaskid,o.servicename,o.servicestackid AS stackid,o.swarmserviceid,o.replicakey AS taskkey,s.created,s.cpuusage,s.memoryactive,s.memorycache,s.memorylimit,s.rxbytes,s.txbytes,s.blockreadbytes,s.blockwritebytes,s.samplecount,s.readcount,s.writecount,s.lastcreated,s.firstcounters,s.lastcounters,s.counterdeltas,s.counterdurations,s.peaks
FROM statisticsobservations s JOIN statisticssources src ON src.id=s.sourceid
JOIN statisticsowners o ON o.id=s.ownerid AND o.kind='Service';

CREATE FUNCTION statistics_capture_open(s bigint, captured bigint) RETURNS boolean LANGUAGE sql VOLATILE AS $$
    SELECT captured>=compacteduntil FROM statisticssources WHERE id=s
$$;

CREATE FUNCTION statistics_delta(current_value double precision, previous_value double precision)
RETURNS double precision LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
 SELECT CASE WHEN current_value IS NULL OR previous_value IS NULL THEN NULL
     WHEN current_value>=previous_value THEN current_value-previous_value ELSE current_value END
$$;
CREATE FUNCTION statistics_sum(a double precision,b double precision)
RETURNS double precision LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
 SELECT CASE WHEN a IS NULL AND b IS NULL THEN NULL ELSE COALESCE(a,0)+COALESCE(b,0) END
$$;

-- Preserve raw host IDs and alert revisions; only processed history is summarized.
CREATE TABLE platformhistorysources (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    platformid uuid NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    nodeid text NOT NULL, -- empty string is the connected host, never a reporting node
    compacteduntil bigint NOT NULL DEFAULT 0,
    UNIQUE(platformid,nodeid)
);
INSERT INTO platformhistorysources(platformid,nodeid)
SELECT platformid,'' FROM platformstats GROUP BY platformid
UNION SELECT platformid,nodeid FROM platformnodeiostats GROUP BY platformid,nodeid;

CREATE TABLE platformgaugerollups (
    sourceid bigint NOT NULL REFERENCES platformhistorysources(id) ON DELETE CASCADE,
    created bigint NOT NULL,
    lastcreated bigint NOT NULL,
    samplecount integer NOT NULL,
    cpuusage double precision NOT NULL,
    memoryusage double precision NOT NULL,
    rxbytes double precision NOT NULL,
    txbytes double precision NOT NULL,
    diskusedbytes numeric,
    disktotalbytes numeric,
    diskusage double precision,
    diskcounts integer[] NOT NULL, -- used bytes, total bytes, usage percent
    PRIMARY KEY(sourceid,created)
) PARTITION BY RANGE(created);
CREATE TABLE platformgaugerollups_default PARTITION OF platformgaugerollups DEFAULT
    WITH (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000);
CREATE INDEX ix_platformgaugerollups_created ON platformgaugerollups(created);

CREATE TABLE platformiorollups (
    sourceid bigint NOT NULL REFERENCES platformhistorysources(id) ON DELETE CASCADE,
    containerid text COLLATE "C" NOT NULL,
    created bigint NOT NULL,
    lastcreated bigint NOT NULL,
    firstcounters double precision[] NOT NULL,
    lastcounters double precision[] NOT NULL,
    counterdeltas double precision[] NOT NULL,
    counterdurations bigint[] NOT NULL,
    blockreadbytes double precision,
    blockwritebytes double precision,
    readcount integer NOT NULL,
    writecount integer NOT NULL,
    PRIMARY KEY(sourceid,created,containerid)
) PARTITION BY RANGE(created);
CREATE TABLE platformiorollups_default PARTITION OF platformiorollups DEFAULT
    WITH (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000);
CREATE INDEX ix_platformiorollups_created ON platformiorollups(created);

CREATE OR REPLACE FUNCTION statistics_partition(parent_name text, day_number bigint) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE child_name text; occupied boolean;
BEGIN
    IF parent_name NOT IN ('statisticssamples','statisticsrollups','platformgaugerollups','platformiorollups') OR day_number<0 THEN
        RAISE EXCEPTION 'Invalid statistics partition';
    END IF;
    child_name := parent_name || '_d' || day_number;
    IF to_regclass('public.' || child_name) IS NOT NULL THEN
        IF NOT EXISTS(SELECT 1 FROM pg_inherits WHERE inhrelid=to_regclass('public.' || child_name)
            AND inhparent=to_regclass('public.' || parent_name)) THEN
            RAISE EXCEPTION 'Statistics partition name belongs to another relation';
        END IF;
        RETURN false;
    END IF;
    EXECUTE format('SELECT EXISTS(SELECT 1 FROM %I WHERE created >= $1 AND created < $2)',parent_name || '_default')
        INTO occupied USING day_number*86400,(day_number+1)*86400;
    -- Never detach a populated default partition or hide its observations.
    IF occupied THEN RETURN false; END IF;
    EXECUTE format('CREATE TABLE %I PARTITION OF %I FOR VALUES FROM (%s) TO (%s)',child_name,parent_name,day_number*86400,(day_number+1)*86400);
    EXECUTE format('ALTER TABLE %I SET (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000)',child_name);
    RETURN true;
END;
$$;
DO $$ DECLARE d bigint; today bigint := floor(extract(epoch FROM now())/86400)::bigint;
BEGIN
    FOR d IN today-8..today+2 LOOP
        PERFORM statistics_partition('platformgaugerollups',d);
        PERFORM statistics_partition('platformiorollups',d);
    END LOOP;
END $$;

-- Reconcile verified host/node projections of the same physical Swarm Task.
CREATE OR REPLACE FUNCTION statistics_source(p uuid, c uuid, t text) RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE existing_source statisticssources; result bigint;
BEGIN
    SELECT * INTO existing_source FROM statisticssources
    WHERE platformid=p AND ((c IS NOT NULL AND containerid=c) OR (t IS NOT NULL AND dockertaskid=t))
    ORDER BY (dockertaskid IS NOT DISTINCT FROM t) DESC LIMIT 1;
    IF FOUND THEN
        -- Host/node inventory can keep two projections of one task during a
        -- same-cluster manager handover. Validate its physical identity against
        -- the persisted Task before attaching the current projection. Deleted
        -- projections can be rediscovered under the same checks. A former host's
        -- NULL node belongs to its already associated Task, not the manager in
        -- today's descriptor; the explicit node of a node projection must match.
        IF c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid
            AND t IS NOT NULL AND t=existing_source.dockertaskid
            AND EXISTS (
                SELECT 1 FROM containers next
                JOIN platforms platform ON platform.id=next.platformid
                JOIN swarmtaskprojections task ON task.platformid=platform.id AND task.dockertaskid=t
                LEFT JOIN containers previous ON previous.id=existing_source.containerid
                WHERE next.id=c AND next.platformid=p
                    AND next.dockercontainerid=task.dockercontainerid
                    AND COALESCE(next.dockernodeid,platform.platformdescriptor->>'nodeID')=task.dockernodeid
                    AND (previous.id IS NULL OR (
                        previous.platformid=p AND previous.dockercontainerid=next.dockercontainerid
                        AND (previous.dockernodeid IS NULL OR previous.dockernodeid=task.dockernodeid)))
            ) THEN
            UPDATE statisticssources SET containerid=c WHERE id=existing_source.id;
            existing_source.containerid := c;
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NOT NULL AND c<>existing_source.containerid)
            THEN
            RAISE EXCEPTION 'Statistics physical identity changed';
        END IF;
        IF (c IS NOT NULL AND existing_source.containerid IS NULL) OR (t IS NOT NULL AND t IS DISTINCT FROM existing_source.dockertaskid) THEN
            UPDATE statisticssources SET containerid=COALESCE(containerid,c),dockertaskid=COALESCE(t,dockertaskid) WHERE id=existing_source.id;
        END IF;
        RETURN existing_source.id;
    END IF;
    INSERT INTO statisticssources(platformid,containerid,dockertaskid) VALUES(p,c,t) RETURNING id INTO result;
    RETURN result;
END;
$$;

-- Typed non-secret instance choices; absence of a property uses its built-in default.
ALTER TABLE instancesetupstates ADD COLUMN instanceconfiguration jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(instanceconfiguration) = 'object');

-- One durable restart claim; retain its audit record until a subsequent Core startup acknowledges it.
CREATE TABLE instancerestarts (
    id integer PRIMARY KEY CHECK (id=1),
    bootid uuid NOT NULL,
    activityid uuid NOT NULL REFERENCES activityevents(id) ON DELETE RESTRICT
);
