-- Keep configuration/lifecycle changes, but only sample enabled health checks.
CREATE OR REPLACE FUNCTION record_container_health_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    latest_id bigint;
    periodic boolean := false;
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
    -- Retain the existing admission lock and installation-wide bound. No other container row is locked.
    PERFORM pg_advisory_xact_lock(680224102);
    INSERT INTO containerhealthtransitions(containerid,platformid,dockernodeid,dockercontainerid,observedat,state,health,origin)
    VALUES(NEW.id,NEW.platformid,NEW.dockernodeid,NEW.dockercontainerid,
        CASE WHEN periodic THEN inspection_ms / 1000 ELSE COALESCE(NEW.projectionobservedat,NEW.updated) END,
        NEW.state,NEW.health,
        CASE WHEN periodic THEN 'Sample' WHEN NEW.health IS NULL THEN 'Lifecycle' ELSE 'Inspection' END)
    RETURNING id INTO latest_id;
    DELETE FROM containerhealthtransitions WHERE id <= latest_id - 100000;
    DELETE FROM containerhealthtransitions WHERE id IN (
        SELECT id FROM containerhealthtransitions WHERE recordedat < now() - interval '7 days' ORDER BY recordedat LIMIT 1000
    );
    IF periodic THEN
        DELETE FROM containerhealthtransitions WHERE id IN (
            SELECT id FROM containerhealthtransitions WHERE containerid=NEW.id AND origin='Sample' ORDER BY id DESC OFFSET 10080
        );
    ELSE
        DELETE FROM containerhealthtransitions WHERE id IN (
            SELECT id FROM containerhealthtransitions WHERE containerid=NEW.id AND origin<>'Sample' ORDER BY id DESC OFFSET 200
        );
    END IF;
    RETURN NEW;
END;
$$;
