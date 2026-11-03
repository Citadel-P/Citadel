SELECT kind, count FROM (
    SELECT 'Deployments' AS kind, count(*) FROM deployments WHERE controlstate IS DISTINCT FROM 'Idle' OR containeroperationid IS NOT NULL OR updatecheckid IS NOT NULL
    UNION ALL SELECT 'Stacks', count(*) FROM stacks WHERE controlstate IS DISTINCT FROM 'Idle' OR containeroperationid IS NOT NULL
    UNION ALL SELECT 'Stack releases', count(*) FROM stackreleases WHERE status='Applying'
    UNION ALL SELECT 'Containers', count(*) FROM containers WHERE controlstate IS DISTINCT FROM 'Idle' OR containeroperationid IS NOT NULL
    UNION ALL SELECT 'Swarm services', count(*) FROM swarmservices WHERE controlstate IS DISTINCT FROM 'Idle' OR updatecheckid IS NOT NULL OR (operationid IS NOT NULL AND (operationstate IS NULL OR operationstate NOT IN ('Completed','Canceled','Rejected','NotAccepted')))
    UNION ALL SELECT 'Node Agent operations', count(*) FROM swarmnodeagentinstallations WHERE operationid IS NOT NULL AND (operationstate IS NULL OR operationstate NOT IN ('Completed','Failed'))
    UNION ALL SELECT 'Git synchronization', count(*) FROM gitrepositoryrefs WHERE status='Syncing'
    UNION ALL SELECT 'Git operations', count(*) FROM gitrepositories WHERE controlstate IS NULL OR controlstate NOT IN ('Idle','Queued')
    UNION ALL SELECT 'Build project operations', count(*) FROM buildprojects WHERE controlstate NOT IN ('Idle','Queued')
    UNION ALL SELECT 'Build pool operations', count(*) FROM buildagentpools WHERE controlstate<>'Idle'
    UNION ALL SELECT 'Script operations', count(*) FROM scripts WHERE controlstate NOT IN ('Idle','Queued')
    UNION ALL SELECT 'Image operations', count(*) FROM images WHERE controlstate IS DISTINCT FROM 'Idle'
    UNION ALL SELECT 'Backup repository operations', count(*) FROM backuprepositories WHERE controlstate NOT IN ('Idle','Queued')
    UNION ALL SELECT 'Backup policy operations', count(*) FROM backuppolicies WHERE controlstate NOT IN ('Idle','Queued')
    UNION ALL SELECT 'Builds', count(*) FROM buildruns WHERE status NOT IN ('Queued','Succeeded','Failed','TimedOut','Cancelled','Rejected','Interrupted')
    UNION ALL SELECT 'Build consumers', count(*) FROM buildcompletionqueue WHERE status<>'Queued'
    UNION ALL SELECT 'Scripts', count(*) FROM scriptruns WHERE status NOT IN ('Queued','Succeeded','Failed','TimedOut','Cancelled','Rejected')
    UNION ALL SELECT 'Backups', count(*) FROM backupruns WHERE status NOT IN ('Queued','Succeeded','SucceededWithWarnings','Failed','TimedOut','Cancelled','Rejected','Interrupted')
    UNION ALL SELECT 'Restores', count(*) FROM backuprestoreruns WHERE status NOT IN ('Queued','Succeeded','SucceededWithWarnings','Failed','TimedOut','Cancelled','Rejected','Interrupted')
    UNION ALL SELECT 'Resource copies', count(*) FROM resourcecopies WHERE status NOT IN ('Review','Succeeded','Failed','Cancelled','Cleaned')
    UNION ALL SELECT 'Stack webhooks', count(*) FROM stackwebhookdeployqueue WHERE status='Processing'
    UNION ALL SELECT 'Alert deliveries', count(*) FROM alertdeliveryoutbox WHERE status='Delivering'
    UNION ALL SELECT 'Build executor reservations', count(*) FROM buildexecutorreservations
    UNION ALL SELECT 'Script executor reservations', count(*) FROM scriptexecutorreservations
    UNION ALL SELECT 'Data executor reservations', count(*) FROM dataexecutorreservations
    UNION ALL SELECT 'Backup repository leases', count(*) FROM backuprepositoryleases
    UNION ALL SELECT 'Backup source leases', count(*) FROM backupsourceleases
) work WHERE count > 0 ORDER BY kind
