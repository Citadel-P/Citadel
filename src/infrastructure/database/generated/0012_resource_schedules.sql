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
