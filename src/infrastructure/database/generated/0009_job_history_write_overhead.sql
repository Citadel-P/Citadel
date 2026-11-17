-- History reads are scoped to retained boot IDs and a job. The primary key
-- already indexes that scope; the extra job/time index amplifies checkpoint WAL.
DROP INDEX jobdiagnosticbuckets_job;
