-- Bounded diagnostics only; these records never authorize worker execution.
CREATE TABLE jobdiagnosticsessions (
    bootid uuid PRIMARY KEY,
    flushsequence bigint NOT NULL,
    coreid uuid NOT NULL,
    version text NOT NULL,
    heartbeat bigint NOT NULL,
    stopped bigint,
    lostevents bigint NOT NULL DEFAULT 0,
    snapshot jsonb NOT NULL
);
CREATE INDEX jobdiagnosticsessions_heartbeat ON jobdiagnosticsessions(heartbeat DESC);
CREATE TABLE jobdiagnosticevents (
    bootid uuid NOT NULL REFERENCES jobdiagnosticsessions(bootid),
    sequence bigint NOT NULL,
    jobid text NOT NULL,
    observed bigint NOT NULL,
    kind text NOT NULL,
    problem jsonb,
    PRIMARY KEY(bootid, sequence)
);
CREATE INDEX jobdiagnosticevents_observed ON jobdiagnosticevents(observed DESC);
CREATE INDEX jobdiagnosticevents_job ON jobdiagnosticevents(jobid, observed DESC);
CREATE TABLE jobdiagnosticbuckets (
    bootid uuid NOT NULL REFERENCES jobdiagnosticsessions(bootid),
    jobid text NOT NULL,
    bucket bigint NOT NULL,
    data jsonb NOT NULL,
    resolutionms bigint NOT NULL DEFAULT 300000
        CHECK (resolutionms IN (300000, 3600000)),
    CONSTRAINT jobdiagnosticbuckets_alignment CHECK (bucket % resolutionms = 0),
    PRIMARY KEY(bootid, jobid, bucket)
);
CREATE INDEX jobdiagnosticbuckets_bucket ON jobdiagnosticbuckets(bucket);
CREATE INDEX jobdiagnosticbuckets_job ON jobdiagnosticbuckets(jobid, bucket DESC);
CREATE INDEX jobdiagnosticbuckets_pending ON jobdiagnosticbuckets(bucket)
    WHERE resolutionms = 300000;
