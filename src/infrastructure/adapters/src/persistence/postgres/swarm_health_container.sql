 ORDER BY task.dockerserviceid,COALESCE(task.slot::text,task.dockernodeid),task.dockercreatedat DESC NULLS LAST,task.versionindex DESC,task.dockertaskid DESC) t
 -- The connected manager uses the platform daemon scope; workers use explicit node scopes.
 -- Resolve only the pinned manager identity, never an arbitrary NULL-node container.
 LEFT JOIN containers c ON c.platformid=t.platformid AND NULLIF(t.dockernodeid,'') IS NOT NULL
 AND c.dockernodeid IS NOT DISTINCT FROM (CASE
 WHEN t.dockernodeid=NULLIF(COALESCE(p.platformdescriptor->>'nodeID',p.platformdescriptor->>'NodeID'),'') THEN NULL
 ELSE t.dockernodeid END) AND c.dockercontainerid=t.dockercontainerid
