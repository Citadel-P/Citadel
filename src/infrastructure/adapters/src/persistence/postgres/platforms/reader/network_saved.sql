SELECT w.kind,w.id ownerid,w.name ownername,w.refs,
 CASE WHEN sum(COALESCE(octet_length(w.compose),0)) OVER (ORDER BY w.kind,w.id)<=4194304 THEN w.compose END compose,
 w.projectname,w.status ownerstatus,COALESCE(w.controlstate='Processing',false) ownerprocessing
FROM authorized w ORDER BY w.kind,w.id LIMIT 2001
