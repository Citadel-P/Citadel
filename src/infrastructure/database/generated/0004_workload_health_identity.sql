-- Retain exact Compose service identity for applied-workload health coverage.
ALTER TABLE containers ADD COLUMN composeservicename text;
