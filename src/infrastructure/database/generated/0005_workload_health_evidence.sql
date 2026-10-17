-- Applied membership is a receipt of a completed full Apply, not a lifecycle status.
ALTER TABLE stacks ADD COLUMN healthappliedspec jsonb;
ALTER TABLE stacks ADD COLUMN healthappliedplatformid uuid;
-- Intent is tied to this container and checked against its observed start time.
ALTER TABLE containers ADD COLUMN healthstoprequestedat bigint;
