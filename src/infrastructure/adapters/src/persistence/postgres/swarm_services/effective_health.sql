CASE
         WHEN s.dockerserviceid IS NULL THEN s.health
         WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'Unknown'
         WHEN lower(COALESCE(projection.updatestate,'')) IN ('paused','rollback_paused','rollback_completed') THEN 'Failed'
         WHEN lower(COALESCE(projection.updatestate,'')) NOT IN ('','none','completed') THEN 'Progressing'
         WHEN projection.desiredtaskcount=0 THEN 'Stopped'
         WHEN projection.runningtaskcount>=projection.desiredtaskcount THEN 'Healthy'
         WHEN projection.runningtaskcount>0 THEN 'Degraded'
         ELSE 'Progressing'
       END
