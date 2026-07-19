ALTER TABLE buildprojects
    ADD COLUMN IF NOT EXISTS webhook jsonb;
