-- Existing rows use their own ID until a relevant input changes. Adding nullable
-- columns avoids rewriting release history or advancing Stack resource versions.
ALTER TABLE stackreleases ADD COLUMN gitcheckrevision uuid;
ALTER TABLE gitrepositoryrefs ADD COLUMN gitcheckrevision uuid;
ALTER TABLE stacks ADD COLUMN checkedgitinputs uuid[];

CREATE FUNCTION advance_stack_git_check_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.gitcheckrevision := gen_random_uuid();
    RETURN NEW;
END $$;

CREATE TRIGGER stack_release_git_check_inputs BEFORE UPDATE OF spec,source ON stackreleases
    FOR EACH ROW WHEN (OLD.spec IS DISTINCT FROM NEW.spec OR OLD.source::jsonb IS DISTINCT FROM NEW.source::jsonb)
    EXECUTE FUNCTION advance_stack_git_check_revision();

-- Pending and Syncing are transient. Recheck a recovered ref even at the same SHA.
CREATE TRIGGER git_ref_stack_check_inputs BEFORE UPDATE OF resolvedcommitsha,status,lasterror,branch,gitrepositoryid ON gitrepositoryrefs
    FOR EACH ROW WHEN (lower(OLD.resolvedcommitsha) IS DISTINCT FROM lower(NEW.resolvedcommitsha)
        OR (OLD.status IS DISTINCT FROM NEW.status AND NEW.status='Degraded')
        OR (NEW.lasterror IS NOT NULL AND NEW.lasterror IS DISTINCT FROM OLD.lasterror)
        OR (OLD.branch,OLD.gitrepositoryid) IS DISTINCT FROM (NEW.branch,NEW.gitrepositoryid))
    EXECUTE FUNCTION advance_stack_git_check_revision();
