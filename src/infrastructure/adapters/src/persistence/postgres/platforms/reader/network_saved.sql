, admitted AS (SELECT * FROM authorized ORDER BY kind,id LIMIT 2001),
sizes AS (
    SELECT w.*, r.xmin::text revision, c.bytes IS NOT NULL cached,
        COALESCE(c.bytes, octet_length(COALESCE(r.spec->>'ComposeFile', r.spec->>'composeFile')), 0) bytes
    FROM admitted w LEFT JOIN stackreleases r ON r.id=w.releaseid
    LEFT JOIN unnest($4::uuid[], $5::text[], $6::int[]) c(id,revision,bytes)
        ON c.id=r.id AND c.revision=r.xmin::text
), budget AS (
    SELECT *, sum(CASE WHEN bytes<=4194304 THEN bytes ELSE 0 END) OVER (ORDER BY kind,id) consumed
    FROM sizes
)
SELECT w.kind,w.id ownerid,w.name ownername,w.refs,w.releaseid,w.revision,w.cached,w.bytes,
    w.bytes<=4194304 AND consumed<=4194304 admitted,
    CASE WHEN w.bytes<=4194304 AND consumed<=4194304 THEN COALESCE(r.spec->>'ComposeFile',r.spec->>'composeFile') END compose,
    COALESCE(r.spec->>'ProjectName',r.spec->>'projectName') projectname,
    w.status ownerstatus,COALESCE(w.controlstate='Processing',false) ownerprocessing
FROM budget w LEFT JOIN stackreleases r ON NOT w.cached AND r.id=w.releaseid ORDER BY w.kind,w.id
