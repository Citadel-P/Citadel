SELECT jsonb_agg(jsonb_build_object(
 'containerId', c.id, 'dockerContainerId', t.dockercontainerid, 'platformId', t.platformid,
 'taskId', t.dockertaskid, 'nodeId', t.dockernodeid, 'name', COALESCE(NULLIF(t.name,''),c.name,t.dockertaskid), 'service', t.dockerserviceid,
 'state', COALESCE(c.state, initcap(t.state)), 'observation', c.health,
 'available', (NOT t.isstale AND c.id IS NOT NULL AND c.projectionstalesince IS NULL
 AND (CASE WHEN c.dockernodeid IS NULL THEN p.status='Online'
 ELSE EXISTS(SELECT 1 FROM swarmnoderuntimeprojectionstates ns WHERE ns.platformid=t.platformid AND ns.dockernodeid=t.dockernodeid AND NOT ns.isstale) END))
 ) ORDER BY t.dockerserviceid,t.slot,t.dockertaskid)
 FROM (SELECT DISTINCT ON (task.dockerserviceid, COALESCE(task.slot::text,task.dockernodeid)) task.*
 FROM swarmtaskprojections task
 WHERE lower(task.desiredstate)='running' AND
