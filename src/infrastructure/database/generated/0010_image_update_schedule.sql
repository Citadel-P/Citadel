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

-- Narrow read models for schedule discovery, admin pages and deadline lookups.
CREATE VIEW git_schedule_branches AS
SELECT repository.id, branches.branch, COALESCE(reference.lastsyncedat,repository.createdat) AS priority,
       COALESCE(reference.lastsyncedat + make_interval(mins => COALESCE(repository.syncintervalminutes,5)), '-infinity') AS due
FROM gitrepositories repository
CROSS JOIN LATERAL (
    SELECT repository.defaultbranch AS branch
    UNION
    SELECT COALESCE(NULLIF(release.spec->>'Branch',''), repository.defaultbranch)
    FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid
    WHERE stack.stacksource='Git' AND release.spec->>'GitRepoId'=repository.id::text
      AND COALESCE(release.spec->>'CommitSha','')=''
) branches
LEFT JOIN gitrepositoryrefs reference ON reference.gitrepositoryid=repository.id AND reference.branch=branches.branch
WHERE branches.branch<>'' AND repository.syncmode='PullInterval' AND repository.controlstate='Idle'
  AND NOT (COALESCE(reference.status='Degraded',false) AND
    (COALESCE(jsonb_array_length(repository.onclone::jsonb->'commands'),0)>0 OR COALESCE(jsonb_array_length(repository.onpull::jsonb->'commands'),0)>0));

CREATE VIEW inventory_image_deadlines AS
SELECT i.id, CASE WHEN i.checkregistrygeneration<g.generation THEN '-infinity'::timestamptz ELSE i.nextupdatecheck END AS due
FROM images i JOIN platforms p ON p.id=i.platformid CROSS JOIN imageupdateregistrygeneration g
WHERE i.controlstate='Idle' AND json_array_length(i.tags)>0 AND p.status='Online'
  AND p.platformdescriptor->>'$type' IN ('Docker','DockerStandalone');

CREATE FUNCTION notify_resource_schedule() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_ARGV[1]='key' THEN
        PERFORM pg_notify('citadel_schedules', TG_ARGV[0]||':'||COALESCE(NEW.id,OLD.id)::text);
    ELSE
        PERFORM pg_notify('citadel_schedules', TG_ARGV[0]);
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER schedule_scripts_membership AFTER INSERT OR DELETE ON scripts FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Script','key');
CREATE TRIGGER schedule_scripts_change AFTER UPDATE OF enabled, scheduleenabled, schedulecron, scheduletimezone, timeoutseconds, name ON scripts FOR EACH ROW WHEN (OLD.enabled IS DISTINCT FROM NEW.enabled OR OLD.scheduleenabled IS DISTINCT FROM NEW.scheduleenabled OR OLD.schedulecron IS DISTINCT FROM NEW.schedulecron OR OLD.scheduletimezone IS DISTINCT FROM NEW.scheduletimezone OR OLD.timeoutseconds IS DISTINCT FROM NEW.timeoutseconds OR OLD.name IS DISTINCT FROM NEW.name) EXECUTE FUNCTION notify_resource_schedule('Script','key');
CREATE TRIGGER schedule_backuppolicies_membership AFTER INSERT OR DELETE ON backuppolicies FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Backup','key');
CREATE TRIGGER schedule_backuppolicies_change AFTER UPDATE OF enabled, archivedat, cron, timezone, backuprepositoryid, name ON backuppolicies FOR EACH ROW WHEN (OLD.enabled IS DISTINCT FROM NEW.enabled OR OLD.archivedat IS DISTINCT FROM NEW.archivedat OR OLD.cron IS DISTINCT FROM NEW.cron OR OLD.timezone IS DISTINCT FROM NEW.timezone OR OLD.backuprepositoryid IS DISTINCT FROM NEW.backuprepositoryid OR OLD.name IS DISTINCT FROM NEW.name) EXECUTE FUNCTION notify_resource_schedule('Backup','key');
CREATE TRIGGER schedule_agentupdatesettings_membership AFTER INSERT OR DELETE ON agentupdatesettings FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Install','family');
CREATE TRIGGER schedule_agentupdatesettings_change AFTER UPDATE OF schedule ON agentupdatesettings FOR EACH ROW WHEN (OLD.schedule IS DISTINCT FROM NEW.schedule) EXECUTE FUNCTION notify_resource_schedule('Install','family');
CREATE TRIGGER schedule_instancesetupstates_membership AFTER INSERT OR DELETE ON instancesetupstates FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('ReleasePolicy','family');
CREATE TRIGGER schedule_instancesetupstates_change AFTER UPDATE OF initializedat, updatechecksenabled, updatecheckintervalhours ON instancesetupstates FOR EACH ROW WHEN (OLD.initializedat IS DISTINCT FROM NEW.initializedat OR OLD.updatechecksenabled IS DISTINCT FROM NEW.updatechecksenabled OR OLD.updatecheckintervalhours IS DISTINCT FROM NEW.updatecheckintervalhours) EXECUTE FUNCTION notify_resource_schedule('ReleasePolicy','family');
CREATE TRIGGER schedule_instanceupdatechecks_membership AFTER INSERT OR DELETE ON instanceupdatechecks FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Release','family');
CREATE TRIGGER schedule_instanceupdatechecks_change AFTER UPDATE OF lastattempt ON instanceupdatechecks FOR EACH ROW WHEN (OLD.lastattempt IS DISTINCT FROM NEW.lastattempt) EXECUTE FUNCTION notify_resource_schedule('Release','family');
CREATE TRIGGER schedule_gitrepositories_membership AFTER INSERT OR DELETE ON gitrepositories FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_gitrepositories_change AFTER UPDATE OF defaultbranch, syncmode, syncintervalminutes, controlstate, onclone, onpull ON gitrepositories FOR EACH ROW WHEN (OLD.defaultbranch IS DISTINCT FROM NEW.defaultbranch OR OLD.syncmode IS DISTINCT FROM NEW.syncmode OR OLD.syncintervalminutes IS DISTINCT FROM NEW.syncintervalminutes OR OLD.controlstate IS DISTINCT FROM NEW.controlstate OR OLD.onclone IS DISTINCT FROM NEW.onclone OR OLD.onpull IS DISTINCT FROM NEW.onpull) EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_stacks_membership AFTER INSERT OR DELETE ON stacks FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_stacks_change AFTER UPDATE OF name,currentstackreleaseid ON stacks FOR EACH ROW WHEN (OLD.name IS DISTINCT FROM NEW.name OR OLD.currentstackreleaseid IS DISTINCT FROM NEW.currentstackreleaseid) EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_images_membership AFTER INSERT OR DELETE ON images FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Image','family');
CREATE TRIGGER schedule_images_change AFTER UPDATE OF tags, registryid, platformid, dockerimageid, controlstate ON images FOR EACH ROW WHEN (OLD.checkrevision IS DISTINCT FROM NEW.checkrevision OR OLD.controlstate IS DISTINCT FROM NEW.controlstate) EXECUTE FUNCTION notify_resource_schedule('Image','family');
CREATE TRIGGER schedule_imageupdateregistrygeneration_change AFTER UPDATE OF generation ON imageupdateregistrygeneration FOR EACH ROW WHEN (OLD.generation IS DISTINCT FROM NEW.generation) EXECUTE FUNCTION notify_resource_schedule('Image','family');
CREATE TRIGGER schedule_platforms_membership AFTER INSERT OR DELETE ON platforms FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Image','family');
CREATE TRIGGER schedule_platforms_change AFTER UPDATE OF status, platformdescriptor ON platforms FOR EACH ROW WHEN (OLD.status IS DISTINCT FROM NEW.status OR OLD.platformdescriptor::jsonb IS DISTINCT FROM NEW.platformdescriptor::jsonb) EXECUTE FUNCTION notify_resource_schedule('Image','family');

CREATE TRIGGER schedule_git_refs_membership AFTER INSERT OR DELETE ON gitrepositoryrefs FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_git_refs_change AFTER UPDATE OF lastsyncedat, status ON gitrepositoryrefs FOR EACH ROW WHEN (OLD.lastsyncedat IS DISTINCT FROM NEW.lastsyncedat OR OLD.status IS DISTINCT FROM NEW.status) EXECUTE FUNCTION notify_resource_schedule('Git','family');

-- Changes to workload definitions refresh admin views; cadence remains system managed.
CREATE TRIGGER schedule_deployments_membership AFTER INSERT OR DELETE ON deployments FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('DeploymentImages','family');
CREATE TRIGGER schedule_deployments_change AFTER UPDATE OF name, spec ON deployments FOR EACH ROW WHEN (OLD.name IS DISTINCT FROM NEW.name OR OLD.spec::jsonb IS DISTINCT FROM NEW.spec::jsonb) EXECUTE FUNCTION notify_resource_schedule('DeploymentImages','family');
CREATE TRIGGER schedule_services_membership AFTER INSERT OR DELETE ON swarmservices FOR EACH ROW EXECUTE FUNCTION notify_resource_schedule('ServiceImages','family');
CREATE TRIGGER schedule_services_change AFTER UPDATE OF name, spec ON swarmservices FOR EACH ROW WHEN (OLD.name IS DISTINCT FROM NEW.name OR OLD.spec::jsonb IS DISTINCT FROM NEW.spec::jsonb) EXECUTE FUNCTION notify_resource_schedule('ServiceImages','family');

-- Presentation changes do not need a deadline lookup.
CREATE TRIGGER schedule_gitrepositories_name AFTER UPDATE OF name ON gitrepositories
    FOR EACH ROW WHEN (OLD.name IS DISTINCT FROM NEW.name) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_services_applied_image AFTER UPDATE OF appliedimagedigest ON swarmservices
    FOR EACH ROW WHEN (OLD.appliedimagedigest IS DISTINCT FROM NEW.appliedimagedigest) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_stack_release_state AFTER UPDATE OF status ON stackreleases
    FOR EACH ROW WHEN (OLD.status IS DISTINCT FROM NEW.status) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');

-- Only the current release contributes branches and rows to the schedule catalog.
DROP TRIGGER schedule_stack_release_state ON stackreleases;
CREATE FUNCTION notify_stack_release_schedule() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    source_type text;
BEGIN
    SELECT stacksource INTO source_type FROM stacks
    WHERE id=NEW.stackid AND currentstackreleaseid=NEW.id;
    IF NOT FOUND THEN RETURN NULL; END IF;

    IF source_type='Git' AND
       (CASE WHEN COALESCE(OLD.spec->>'CommitSha','')='' THEN
            ARRAY[OLD.spec->>'GitRepoId',NULLIF(OLD.spec->>'Branch','')] END)
       IS DISTINCT FROM
       (CASE WHEN COALESCE(NEW.spec->>'CommitSha','')='' THEN
            ARRAY[NEW.spec->>'GitRepoId',NULLIF(NEW.spec->>'Branch','')] END)
    THEN
        -- One family wake recomputes the minimum across former and new repositories.
        PERFORM pg_notify('citadel_schedules','Git');
    ELSE
        PERFORM pg_notify('citadel_schedules','Catalog');
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER schedule_stack_release_change AFTER UPDATE OF spec,platformid,status ON stackreleases
    FOR EACH ROW WHEN (
        (OLD.platformid, OLD.status, OLD.spec->>'GitRepoId', OLD.spec->>'Branch', OLD.spec->>'CommitSha',
         OLD.spec->>'RegistryId', OLD.spec->>'UpdateBehavior') IS DISTINCT FROM
        (NEW.platformid, NEW.status, NEW.spec->>'GitRepoId', NEW.spec->>'Branch', NEW.spec->>'CommitSha',
         NEW.spec->>'RegistryId', NEW.spec->>'UpdateBehavior')
    ) EXECUTE FUNCTION notify_stack_release_schedule();

DROP TRIGGER schedule_stacks_change ON stacks;
CREATE TRIGGER schedule_stacks_change AFTER UPDATE OF stacksource,currentstackreleaseid ON stacks
    FOR EACH ROW WHEN ((OLD.stacksource,OLD.currentstackreleaseid) IS DISTINCT FROM (NEW.stacksource,NEW.currentstackreleaseid))
    EXECUTE FUNCTION notify_resource_schedule('Git','family');
CREATE TRIGGER schedule_stacks_name AFTER UPDATE OF name ON stacks
    FOR EACH ROW WHEN (OLD.name IS DISTINCT FROM NEW.name) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_deployment_platform AFTER UPDATE OF platformid ON deployments
    FOR EACH ROW WHEN (OLD.platformid IS DISTINCT FROM NEW.platformid) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_service_platform AFTER UPDATE OF platformid ON swarmservices
    FOR EACH ROW WHEN (OLD.platformid IS DISTINCT FROM NEW.platformid) EXECUTE FUNCTION notify_resource_schedule('Catalog','family');

-- Fence the due observation against the version obtained by the repository row lock.
CREATE OR REPLACE VIEW git_schedule_branches AS
SELECT repository.id, branches.branch, COALESCE(reference.lastsyncedat,repository.createdat) AS priority,
       COALESCE(reference.lastsyncedat + make_interval(mins => COALESCE(repository.syncintervalminutes,5)), '-infinity') AS due,
       repository.xmin AS repository_version
FROM gitrepositories repository
CROSS JOIN LATERAL (
    SELECT repository.defaultbranch AS branch
    UNION
    SELECT COALESCE(NULLIF(release.spec->>'Branch',''), repository.defaultbranch)
    FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid
    WHERE stack.stacksource='Git' AND release.spec->>'GitRepoId'=repository.id::text
      AND COALESCE(release.spec->>'CommitSha','')=''
) branches
LEFT JOIN gitrepositoryrefs reference ON reference.gitrepositoryid=repository.id AND reference.branch=branches.branch
WHERE branches.branch<>'' AND repository.syncmode='PullInterval' AND repository.controlstate='Idle'
  AND NOT (COALESCE(reference.status='Degraded',false) AND
    (COALESCE(jsonb_array_length(repository.onclone::jsonb->'commands'),0)>0 OR COALESCE(jsonb_array_length(repository.onpull::jsonb->'commands'),0)>0));

-- Only a scheduled run changing phase affects the schedule catalog.
CREATE TRIGGER schedule_script_run_progress AFTER UPDATE OF status ON scriptruns
    FOR EACH ROW WHEN (NEW.trigger='Schedule' AND OLD.status IS DISTINCT FROM NEW.status)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_backup_run_progress AFTER UPDATE OF status ON backupruns
    FOR EACH ROW WHEN (NEW.trigger='Schedule' AND OLD.status IS DISTINCT FROM NEW.status)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');

-- Find the most recent retained scheduled run for each visible resource.
CREATE INDEX ix_scriptruns_latest_scheduled ON scriptruns(scriptid,queuedat DESC,id DESC) WHERE trigger='Schedule';
CREATE INDEX ix_backupruns_latest_scheduled ON backupruns(backuppolicyid,queuedat DESC,id DESC) WHERE trigger='Schedule';

-- Resource dot changes refresh the catalog without rescheduling work.
CREATE TRIGGER schedule_deployment_status AFTER UPDATE OF status ON deployments
    FOR EACH ROW WHEN (OLD.status IS DISTINCT FROM NEW.status)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_service_health AFTER UPDATE OF health ON swarmservices
    FOR EACH ROW WHEN (OLD.health IS DISTINCT FROM NEW.health)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_git_status AFTER UPDATE OF status ON gitrepositories
    FOR EACH ROW WHEN (OLD.status IS DISTINCT FROM NEW.status)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
CREATE TRIGGER schedule_image_usage AFTER UPDATE OF containers ON images
    FOR EACH ROW WHEN ((OLD.containers > 0) <> (NEW.containers > 0) AND json_array_length(NEW.tags) > 0)
    EXECUTE FUNCTION notify_resource_schedule('Catalog','family');
