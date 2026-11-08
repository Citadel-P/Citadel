-- Typed non-secret instance choices; absence of a property uses its built-in default.
ALTER TABLE instancesetupstates ADD COLUMN instanceconfiguration jsonb NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(instanceconfiguration) = 'object');
