WITH candidates AS MATERIALIZED (
    SELECT r.id, r.completedat
    FROM buildruns r
    WHERE r.completedat < CURRENT_TIMESTAMP - make_interval(days => $1)
      AND (r.completedat, r.id) > (coalesce($2::timestamptz, '-infinity'), coalesce($3::uuid, '00000000-0000-0000-0000-000000000000'))
      AND r.status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted')
    ORDER BY r.completedat, r.id
    LIMIT 1000 FOR UPDATE SKIP LOCKED
), removed AS (
    DELETE FROM buildruns r USING candidates c WHERE r.id=c.id
      AND NOT EXISTS (SELECT 1 FROM buildexecutorreservations b WHERE b.runid=c.id)
      AND NOT EXISTS (SELECT 1 FROM buildcompletionqueue q WHERE q.buildrunid=c.id)
      AND NOT EXISTS (SELECT 1 FROM buildprojects p WHERE p.currentrunid=c.id)
      AND NOT EXISTS (SELECT 1 FROM buildspecassignments ref WHERE ref.referenceid=c.id)
    RETURNING r.id
)
SELECT (SELECT count(*) FROM removed) AS deleted, last.completedat, last.id
FROM (SELECT count(*) AS visited FROM candidates) page
LEFT JOIN LATERAL (
    SELECT completedat,id FROM candidates WHERE page.visited=1000
    ORDER BY completedat DESC,id DESC LIMIT 1
) last ON true
