-- A registry edit can change credential selection even for unassociated images.
-- Advancing one generation makes the old index range due without rewriting inventory.
CREATE TABLE imageupdateregistrygeneration (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    generation bigint NOT NULL DEFAULT 0
);
INSERT INTO imageupdateregistrygeneration DEFAULT VALUES;

ALTER TABLE images
    ADD COLUMN checkregistrygeneration bigint NOT NULL DEFAULT 0,
    ADD COLUMN nextupdatecheck timestamptz NOT NULL DEFAULT '-infinity',
    ADD COLUMN checkrevision bigint NOT NULL DEFAULT 0;
CREATE INDEX images_update_due ON images(checkregistrygeneration, nextupdatecheck, id)
    WHERE controlstate='Idle' AND json_array_length(tags)>0;

CREATE FUNCTION advance_image_registry_generation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE imageupdateregistrygeneration SET generation=generation+1;
    RETURN NULL;
END $$;
CREATE TRIGGER image_registry_membership AFTER INSERT OR DELETE ON registries
    FOR EACH ROW EXECUTE FUNCTION advance_image_registry_generation();
CREATE TRIGGER image_registry_revision AFTER UPDATE ON registries
    FOR EACH ROW WHEN (OLD.registryhost IS DISTINCT FROM NEW.registryhost
        OR OLD.configuration::jsonb IS DISTINCT FROM NEW.configuration::jsonb
        OR (OLD.status='Disabled') IS DISTINCT FROM (NEW.status='Disabled'))
    EXECUTE FUNCTION advance_image_registry_generation();

CREATE FUNCTION invalidate_image_update_schedule() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.nextupdatecheck := '-infinity';
    NEW.checkrevision := OLD.checkrevision+1;
    RETURN NEW;
END $$;
CREATE TRIGGER image_check_identity BEFORE UPDATE OF tags, registryid, platformid, dockerimageid ON images
    FOR EACH ROW WHEN (OLD.tags::jsonb IS DISTINCT FROM NEW.tags::jsonb
        OR OLD.registryid IS DISTINCT FROM NEW.registryid OR OLD.platformid<>NEW.platformid
        OR OLD.dockerimageid<>NEW.dockerimageid)
    EXECUTE FUNCTION invalidate_image_update_schedule();
