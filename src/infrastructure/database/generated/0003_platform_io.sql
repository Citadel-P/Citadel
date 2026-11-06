-- Retain physical container I/O counters with platform telemetry, independently of inventory.
ALTER TABLE platformstats ADD COLUMN containerio jsonb;
