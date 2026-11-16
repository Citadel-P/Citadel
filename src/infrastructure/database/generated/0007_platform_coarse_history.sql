-- Optional fifteen-minute I/O summaries. Minute rows remain authoritative for
-- fine charts and partial boundaries; this watermark never fences captures.
ALTER TABLE platformhistorysources ADD COLUMN coarseneduntil bigint NOT NULL DEFAULT 0;

CREATE TABLE platformiocoarserollups (
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
CREATE TABLE platformiocoarserollups_default PARTITION OF platformiocoarserollups DEFAULT
    WITH (autovacuum_vacuum_scale_factor=0.05,autovacuum_vacuum_threshold=1000);
CREATE INDEX ix_platformiocoarserollups_created ON platformiocoarserollups(created);

CREATE OR REPLACE FUNCTION statistics_partition(parent_name text, day_number bigint) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE child_name text; occupied boolean;
BEGIN
    IF parent_name NOT IN ('statisticssamples','statisticsrollups','platformgaugerollups','platformiorollups','platformiocoarserollups') OR day_number<0 THEN
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
        PERFORM statistics_partition('platformiocoarserollups',d);
    END LOOP;
END $$;
