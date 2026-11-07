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
