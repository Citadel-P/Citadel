-- Index canonical UUID string values, including nested values in retained releases.
-- A spec may mention an ID before the corresponding run exists.
CREATE TABLE buildspecassignments (
    ownertable regclass NOT NULL,
    ownerid uuid NOT NULL,
    referenceid uuid NOT NULL,
    PRIMARY KEY (ownertable, ownerid, referenceid)
);
CREATE INDEX ix_buildspecassignments_reference ON buildspecassignments(referenceid);

CREATE FUNCTION build_spec_ids(document jsonb) RETURNS SETOF uuid
LANGUAGE sql IMMUTABLE STRICT PARALLEL SAFE AS $$
    SELECT DISTINCT (value #>> '{}')::uuid
    FROM jsonb_path_query(document, 'strict $.** ? (@.type() == "string")') value
    WHERE (value #>> '{}') COLLATE "C" ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
$$;

CREATE FUNCTION record_build_spec_ids() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.id=OLD.id AND NEW.spec=OLD.spec THEN
        RETURN NULL;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        DELETE FROM buildspecassignments WHERE ownertable=TG_RELID AND ownerid=OLD.id;
    END IF;
    IF TG_OP <> 'DELETE' THEN
        INSERT INTO buildspecassignments SELECT TG_RELID, NEW.id, build_spec_ids(NEW.spec);
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER deployment_build_references AFTER INSERT OR UPDATE OF spec,id OR DELETE ON deployments
    FOR EACH ROW EXECUTE FUNCTION record_build_spec_ids();
CREATE TRIGGER stack_release_build_references AFTER INSERT OR UPDATE OF spec,id OR DELETE ON stackreleases
    FOR EACH ROW EXECUTE FUNCTION record_build_spec_ids();

INSERT INTO buildspecassignments SELECT 'deployments'::regclass, id, build_spec_ids(spec) FROM deployments;
INSERT INTO buildspecassignments SELECT 'stackreleases'::regclass, id, build_spec_ids(spec) FROM stackreleases;

-- Retention pages use the run ID to resume within equal completion timestamps.
DROP INDEX ix_buildruns_retention;
CREATE INDEX ix_buildruns_retention ON buildruns(completedat,id)
    WHERE status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted');
