-- Indexed per-container ordinals replace an OFFSET walk on every health write.
-- Preserve applied history, exact event admission and synchronous storage caps.
ALTER TABLE containerhealthtransitions ADD COLUMN retentionordinal bigint;
WITH numbered AS (
    SELECT id,row_number() OVER(PARTITION BY containerid,(origin='Sample') ORDER BY id) ordinal
    FROM containerhealthtransitions
)
UPDATE containerhealthtransitions h SET retentionordinal=n.ordinal FROM numbered n WHERE h.id=n.id;
ALTER TABLE containerhealthtransitions ALTER COLUMN retentionordinal SET NOT NULL;
CREATE TABLE containerhealthhistorycounters (
    containerid uuid PRIMARY KEY,
    platformid uuid NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    lastid bigint NOT NULL,
    samples bigint NOT NULL DEFAULT 0,
    changes bigint NOT NULL DEFAULT 0
);
INSERT INTO containerhealthhistorycounters(containerid,platformid,lastid,samples,changes)
SELECT containerid,platformid,max(id),COALESCE(max(retentionordinal) FILTER(WHERE origin='Sample'),0),
    COALESCE(max(retentionordinal) FILTER(WHERE origin<>'Sample'),0)
FROM containerhealthtransitions GROUP BY containerid,platformid;
CREATE INDEX ix_containerhealthhistorycounters_lastid ON containerhealthhistorycounters(lastid);
DROP INDEX ix_containerhealthtransitions_samples;
DROP INDEX ix_containerhealthtransitions_changes;
CREATE INDEX ix_containerhealthtransitions_sample_ordinal ON containerhealthtransitions(containerid,retentionordinal) WHERE origin='Sample';
CREATE INDEX ix_containerhealthtransitions_change_ordinal ON containerhealthtransitions(containerid,retentionordinal) WHERE origin<>'Sample';

CREATE FUNCTION number_container_health_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- Keep the existing installation-wide admission ordering. The critical
    -- section now performs indexed tail deletion rather than a retained scan.
    PERFORM pg_advisory_xact_lock(680224102);
    INSERT INTO containerhealthhistorycounters(containerid,platformid,lastid,samples,changes)
    VALUES(NEW.containerid,NEW.platformid,NEW.id,(NEW.origin='Sample')::integer,(NEW.origin<>'Sample')::integer)
    ON CONFLICT(containerid) DO UPDATE SET
        lastid=EXCLUDED.lastid,
        samples=containerhealthhistorycounters.samples+EXCLUDED.samples,
        changes=containerhealthhistorycounters.changes+EXCLUDED.changes
    RETURNING CASE WHEN NEW.origin='Sample' THEN samples ELSE changes END INTO NEW.retentionordinal;
    RETURN NEW;
END;
$$;
CREATE TRIGGER container_health_history_number BEFORE INSERT ON containerhealthtransitions
FOR EACH ROW EXECUTE FUNCTION number_container_health_history();

CREATE OR REPLACE FUNCTION record_container_health_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    latest_id bigint;
    periodic boolean := false;
    ordinal bigint;
    inspection_ms bigint := (NEW.health->>'observedAt')::bigint;
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.state IS NOT DISTINCT FROM OLD.state
        AND (NEW.health->'configuration', NEW.health->'result', NEW.health->'oomKilled', NEW.health->'exitCode')
            IS NOT DISTINCT FROM
            (OLD.health->'configuration', OLD.health->'result', OLD.health->'oomKilled', OLD.health->'exitCode') THEN
        -- Statistics, repeated delivery, stale evidence and heartbeats cannot create samples.
        -- Admit at most one unchanged inspection per UTC minute, without synthesizing missed minutes.
        IF NEW.health->>'configuration' IS DISTINCT FROM 'Enabled'
            OR inspection_ms IS NULL
            OR inspection_ms / 60000 <= COALESCE((OLD.health->>'observedAt')::bigint / 60000, inspection_ms / 60000)
            OR inspection_ms NOT BETWEEN floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint - 180000
                AND floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint THEN
            RETURN NEW;
        END IF;
        periodic := true;
    END IF;
    -- Acquire before identity allocation to preserve global sequence ordering.
    PERFORM pg_advisory_xact_lock(680224102);
    INSERT INTO containerhealthtransitions(containerid,platformid,dockernodeid,dockercontainerid,observedat,state,health,origin)
    VALUES(NEW.id,NEW.platformid,NEW.dockernodeid,NEW.dockercontainerid,
        CASE WHEN periodic THEN inspection_ms / 1000 ELSE COALESCE(NEW.projectionobservedat,NEW.updated) END,
        NEW.state,NEW.health,
        CASE WHEN periodic THEN 'Sample' WHEN NEW.health IS NULL THEN 'Lifecycle' ELSE 'Inspection' END)
    RETURNING id,retentionordinal INTO latest_id,ordinal;
    DELETE FROM containerhealthtransitions WHERE id <= latest_id - 100000;
    DELETE FROM containerhealthhistorycounters WHERE lastid <= latest_id - 100000;
    -- Age expiry belongs to the existing lease-owned maintenance sweep. Reads
    -- still enforce seven days, and the synchronous global/local caps remain.
    IF periodic THEN
        DELETE FROM containerhealthtransitions WHERE id IN (
            SELECT id FROM containerhealthtransitions WHERE containerid=NEW.id AND origin='Sample' AND retentionordinal<=ordinal-10080
        );
    ELSE
        DELETE FROM containerhealthtransitions WHERE id IN (
            SELECT id FROM containerhealthtransitions WHERE containerid=NEW.id AND origin<>'Sample' AND retentionordinal<=ordinal-200
        );
    END IF;
    RETURN NEW;
END;
$$;
