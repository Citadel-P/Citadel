 FROM (SELECT DISTINCT ON (task.dockerserviceid, COALESCE(task.slot::text,task.dockernodeid)) task.*
 FROM swarmtaskprojections task
 WHERE lower(task.desiredstate)='running' AND
