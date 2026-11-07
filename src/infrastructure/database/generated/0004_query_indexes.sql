-- Match the global Activities order and bounded retention predicates.
CREATE INDEX ix_activityevents_createdat_id ON activityevents(createdat DESC, id DESC);
CREATE INDEX ix_scriptruns_retention ON scriptruns(finishedat)
    WHERE status NOT IN ('Queued','Running');
CREATE INDEX ix_buildruns_retention ON buildruns(completedat)
    WHERE status IN ('Succeeded','Failed','TimedOut','Cancelled','Interrupted');
CREATE INDEX ix_resourcecopies_review_expiry ON resourcecopies(createdat)
    WHERE status='Review';
