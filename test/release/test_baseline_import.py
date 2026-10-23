"""Real offline imports; opt in with a disposable PostgreSQL container only."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import uuid


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("baseline_import", ROOT / "src/tools/release/baseline_import.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class ImportSafetyTests(unittest.TestCase):
    def test_identifier_validation_rejects_sql_and_shell_fragments(self):
        for value in ["public.users", "-flag", "db;delete", 'db"', "$(cmd)", ""]:
            with self.assertRaises(ValueError):
                importer.identifier(value)
        self.assertEqual(importer.identifier("fixture_123"), '"fixture_123"')


@unittest.skipUnless(os.environ.get("CITADEL_BASELINE_IMPORT_CONTAINER"),
                     "requires an explicitly supplied disposable PostgreSQL container")
class BaselineImportTests(unittest.TestCase):
    def setUp(self):
        self.container = os.environ["CITADEL_BASELINE_IMPORT_CONTAINER"]
        info = json.loads(subprocess.check_output(["docker", "inspect", self.container]))[0]
        env = dict(item.split("=", 1) for item in info["Config"]["Env"] if "=" in item)
        self.user = env["POSTGRES_USER"]
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        baseline = (ROOT / "src/infrastructure/database/generated/0001_initial.sql").read_text()
        old = baseline.replace("    lastobservedat timestamp with time zone,\n", "")
        self.assertEqual(hashlib.sha256(old.encode()).hexdigest(), importer.OLD_BASELINE)
        self.assertEqual(hashlib.sha256(baseline.encode()).hexdigest(), importer.NEW_BASELINE)
        databases = []
        for sql, checksum in [(old, importer.OLD_BASELINE), (baseline, importer.NEW_BASELINE)]:
            name = "import_fixture_" + uuid.uuid4().hex
            subprocess.run(["docker", "exec", self.container, "createdb", "-U", self.user, name], check=True)
            self.addCleanup(subprocess.run, ["docker", "exec", self.container, "dropdb", "-U", self.user, "--force", name], check=True)
            database = importer.Database(self.container, name)
            # Fixture journals reproduce real runner state without starting Core workers.
            journal = "CREATE TABLE citadel_schema_migrations(id text PRIMARY KEY,name text NOT NULL,checksum text NOT NULL,started_at timestamptz NOT NULL,completed_at timestamptz,failure text);"
            database.sql(sql + journal + f"INSERT INTO citadel_schema_migrations VALUES('0001','initial','{checksum}',now(),now(),NULL)")
            databases.append(database)
        self.source, self.target = databases
        self.report = Path(self.temporary.name) / "report.json"
        self.source.sql("""
INSERT INTO alertrulestates(alertruleid,resourceid,consecutivematches,createdbyactorid,lasttriggeredat)
SELECT id,'00000000-0000-0000-0000-000000000123',7,'00000000-0000-0000-0000-000000000001','2026-10-23T10:00:00Z'
FROM alertrules WHERE type='DeploymentImageUpdateAvailable' LIMIT 1;
UPDATE alertrules SET name='Preserved customized rule',cooldownseconds=123 WHERE type='DeploymentImageUpdateAvailable';
""")

    def test_import_preserves_customization_and_initializes_observation_without_rewriting_journals(self):
        before_source = self.source.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j")
        before_target = self.target.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j")
        importer.convert(self.source, self.target, self.report)
        self.assertEqual(self.source.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j"), before_source)
        self.assertEqual(self.target.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j"), before_target)
        self.assertEqual(self.target.sql("SELECT bool_and(lastobservedat=lasttriggeredat AND consecutivematches=7) FROM alertrulestates"), "t")
        self.assertEqual(self.target.sql("SELECT name||':'||cooldownseconds FROM alertrules WHERE type='DeploymentImageUpdateAvailable'"), "Preserved customized rule:123")
        self.assertTrue(json.loads(self.report.read_text())["foreignKeysValidated"])

    def test_invalid_import_rolls_back_seed_replacement_and_backfill(self):
        self.source.sql("SET session_replication_role=replica; UPDATE alertrulestates SET createdbyactorid='00000000-0000-0000-0000-000000000999'; SET session_replication_role=origin")
        before = self.target.sql("SELECT json_agg(r ORDER BY id) FROM alertrules r")
        with self.assertRaisesRegex(RuntimeError, "rolled back"):
            importer.convert(self.source, self.target, self.report)
        self.assertEqual(self.target.sql("SELECT json_agg(r ORDER BY id) FROM alertrules r"), before)
        self.assertEqual(self.target.sql("SELECT count(*) FROM alertrulestates"), "0")
        self.assertFalse(self.report.exists())

    def test_schema_drift_is_rejected_before_import(self):
        self.source.sql("ALTER TABLE alertrules ADD COLUMN unsupported text")
        with self.assertRaisesRegex(ValueError, "Schema columns differ"):
            importer.convert(self.source, self.target, self.report)
        self.assertFalse(self.report.exists())


if __name__ == "__main__":
    unittest.main()
