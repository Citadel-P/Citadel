-- Container health observations and bounded transition history; existing rows remain unknown.
ALTER TABLE containers ADD COLUMN health jsonb;

CREATE TABLE containerhealthtransitions (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    containerid uuid NOT NULL,
    platformid uuid NOT NULL REFERENCES platforms(id) ON DELETE CASCADE,
    dockernodeid text,
    dockercontainerid text NOT NULL,
    observedat bigint NOT NULL,
    recordedat timestamptz NOT NULL DEFAULT now(),
    state text NOT NULL,
    health jsonb,
    origin text NOT NULL
);
CREATE INDEX ix_containerhealthtransitions_container ON containerhealthtransitions(containerid, id DESC);
CREATE INDEX ix_containerhealthtransitions_retention ON containerhealthtransitions(recordedat);

CREATE FUNCTION record_container_health_transition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE latest_id bigint;
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.state IS NOT DISTINCT FROM OLD.state
        AND (NEW.health->'configuration', NEW.health->'result', NEW.health->'oomKilled', NEW.health->'exitCode')
            IS NOT DISTINCT FROM
            (OLD.health->'configuration', OLD.health->'result', OLD.health->'oomKilled', OLD.health->'exitCode') THEN
        RETURN NEW;
    END IF;
    -- Only transitions take this lock. Never lock another container row while holding it.
    -- Serializing sequence admission makes the installation-wide cap exact under concurrency.
    PERFORM pg_advisory_xact_lock(680224102);
    INSERT INTO containerhealthtransitions(containerid,platformid,dockernodeid,dockercontainerid,observedat,state,health,origin)
    VALUES(NEW.id,NEW.platformid,NEW.dockernodeid,NEW.dockercontainerid,COALESCE(NEW.projectionobservedat,NEW.updated),NEW.state,NEW.health,
        CASE WHEN NEW.health IS NULL THEN 'Lifecycle' ELSE 'Inspection' END)
    RETURNING id INTO latest_id;
    DELETE FROM containerhealthtransitions WHERE id <= latest_id - 100000;
    DELETE FROM containerhealthtransitions WHERE id IN (SELECT id FROM containerhealthtransitions WHERE recordedat < now() - interval '7 days' ORDER BY recordedat LIMIT 1000);
    DELETE FROM containerhealthtransitions WHERE id IN (
        SELECT id FROM containerhealthtransitions WHERE containerid=NEW.id ORDER BY id DESC OFFSET 200
    );
    RETURN NEW;
END;
$$;
CREATE TRIGGER container_health_transition AFTER INSERT OR UPDATE OF state,health ON containers
    FOR EACH ROW EXECUTE FUNCTION record_container_health_transition();

CREATE FUNCTION invalidate_container_health_on_state_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.state IS DISTINCT FROM OLD.state AND NEW.health IS NOT DISTINCT FROM OLD.health THEN
        NEW.health = NULL;
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER container_health_execution BEFORE UPDATE OF state ON containers
    FOR EACH ROW EXECUTE FUNCTION invalidate_container_health_on_state_change();
