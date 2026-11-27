INSERT INTO backuppolicies(id,backuprepositoryid,createdbyactorid,name,normalizedname,runasactorid,source)
SELECT gen_random_uuid(),backuprepositoryid,createdbyactorid,'Archive','history-archive',runasactorid,source
FROM backuppolicies WHERE normalizedname='r4-payments-daily';

-- The newest queued failure completed before the success. A later cancellation is ignored.
INSERT INTO backupruns(id,backuppolicyid,backuprepositoryid,policynamesnapshot,repositorytypesnapshot,
    snapshotavailability,sourcesnapshot,status,trigger,triggeredbyactorid,queuedat,completedat)
SELECT gen_random_uuid(),p.id,p.backuprepositoryid,p.name,'Local','Unavailable','{}',
    CASE WHEN n=4001 THEN 'Cancelled' ELSE 'Failed' END,'Scheduled',p.runasactorid,
    now()-interval '3 days'+n*interval '1 second',
    CASE WHEN n=4001 THEN now()+interval '1 hour' ELSE now()-interval '3 days'+n*interval '2 seconds' END
FROM backuppolicies p CROSS JOIN generate_series(1,4001) n WHERE p.normalizedname IN ('r4-payments-daily','history-archive');
UPDATE backupruns SET queuedat=now()-interval '10 days' WHERE status='Succeeded' AND policynamesnapshot='payments-daily';

INSERT INTO alertevents(id,alertruleid,deduplicationkey,info,resourcetype,resourceid,resourcename,severity,type,resolvedat)
SELECT gen_random_uuid(),e.alertruleid,e.id::text||n,'{}',e.resourcetype,e.resourceid,e.resourcename,e.severity,e.type,now()
FROM alertevents e CROSS JOIN generate_series(1,5000) n WHERE e.deduplicationkey IN ('r4-host','r4-workload','r4-update');
ANALYZE backupruns;
ANALYZE backuppolicies;
ANALYZE alertevents;
