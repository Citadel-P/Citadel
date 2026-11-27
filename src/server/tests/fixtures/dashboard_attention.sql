DO $fixture$
DECLARE
    actor_id uuid := current_setting('fixture.actor')::uuid;
    host_id uuid := current_setting('fixture.platform')::uuid;
    failed_id uuid := current_setting('fixture.deployment')::uuid;
    stack_id uuid := gen_random_uuid();
    release_id uuid := gen_random_uuid();
    secret_id uuid := gen_random_uuid();
    repo_id uuid := gen_random_uuid();
    policy_id uuid := gen_random_uuid();
    rule_id uuid := gen_random_uuid();
    tag_id uuid;
BEGIN
    UPDATE platforms SET status='Offline' WHERE id=host_id;
    UPDATE deployments SET spec='{"Image":{"ResolvedDigest":"current"}}',
        autoupdatestate_status='UpdateAvailable',autoupdatestate_currentdigest='current',autoupdatestate_remotedigest='remote',
        status=CASE WHEN id=failed_id THEN 'Failed' WHEN name='app-2' THEN 'Degraded' ELSE 'Healthy' END
    WHERE platformid=host_id AND (id=failed_id OR name IN (SELECT 'app-'||n FROM generate_series(2,11) n));
    INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate)
    VALUES(stack_id,'web-stack',actor_id,'{}','WebEditor',json_build_object('RecreateStackOnNewImageState',
        json_build_object('AutoUpdateStates',json_build_array(json_build_object('UpdateAvailable',true,'CurrentDigest','old',
        'RemoteDigest','new','LastCheckedAt',now())))));
    INSERT INTO stackreleases(id,stackid,platformid,createdbyactorid,spec,status,version)
    VALUES(release_id,stack_id,host_id,actor_id,'{}','Healthy','1');
    UPDATE stacks SET currentstackreleaseid=release_id WHERE id=stack_id;
    INSERT INTO secretdefinitions(id,name,providertype) VALUES(secret_id,'Fixture repository password','Internal');
    INSERT INTO backuprepositories(id,name,normalizedname,passwordsecretid,spec,status,type,createdbyactorid)
    VALUES(repo_id,'R4 fixture','r4-fixture',secret_id,'{}','Initialized','Local',actor_id);
    INSERT INTO backuppolicies(id,backuprepositoryid,createdbyactorid,name,normalizedname,runasactorid,source)
    VALUES(policy_id,repo_id,actor_id,'payments-daily','r4-payments-daily',actor_id,'{}');
    INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,snapshotavailability,
        sourcesnapshot,status,trigger,triggeredbyactorid,completedat)
    VALUES(gen_random_uuid(),policy_id,repo_id,'payments-daily','Local','Unavailable','{}','Failed','Scheduled',actor_id,now()-interval '1 day');
    INSERT INTO alertrules(id,createdbyactorid,limitedto,name,quiethours,severity,type)
    VALUES(rule_id,actor_id,'[]','Fixture alerts','[]','Warning','DeploymentImageUpdateAvailable');
    INSERT INTO alertevents(id,alertruleid,deduplicationkey,info,resourcetype,resourceid,resourcename,severity,type,acknowledgedat)
    VALUES(gen_random_uuid(),rule_id,'r4-host','{}','Platform',host_id,'Fixture host','Critical','PlatformOffline',now()),
          (gen_random_uuid(),rule_id,'r4-workload','{}','Deployment',failed_id,'Fixture workload','Warning','DeploymentFailed',NULL),
          (gen_random_uuid(),rule_id,'r4-update','{}','Stack',stack_id,'web-stack','Warning','StackImageUpdateAvailable',NULL);
    FOREACH tag_id IN ARRAY ARRAY[gen_random_uuid(),gen_random_uuid()] LOOP
        INSERT INTO tags(id,name,normalizedname,createdbyactorid,color)
        VALUES(tag_id,'fixture-'||tag_id,'fixture-'||tag_id,actor_id,'#888888');
        INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid)
        VALUES('Deployment',failed_id,tag_id,actor_id);
        IF NOT EXISTS (SELECT 1 FROM resourcetags WHERE resourcetype='Platform' AND resourceid=host_id) THEN
            INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('Platform',host_id,tag_id,actor_id);
        END IF;
    END LOOP;
END
$fixture$;
