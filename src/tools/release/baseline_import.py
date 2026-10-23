#!/usr/bin/env python3
"""Offline 0.1.1 data conversion between disposable PostgreSQL databases.

Restore the old dump into a source copy and initialize a separate target with
the new Core's `migrate` command first. Never point this tool at a live database.
The PostgreSQL container's default database is explicitly protected.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


OLD_BASELINE = "c622e2f16a0b11b3ff2a6021c1f097f7b97cd575fdfb241a63466ef1fc444d05"
NEW_BASELINE = "75c219301520bcb5c5a9904526334a5fe5f45cb57bc554c56062d3b185b4afa0"
JOURNAL = "citadel_schema_migrations"


def identifier(value):
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", value):
        raise ValueError("Only simple PostgreSQL identifiers are supported")
    return '"' + value + '"'


class Database:
    def __init__(self, container, name):
        identifier(name)
        if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]*", container):
            raise ValueError("Select an explicit Docker container name or ID")
        self.container, self.name = container, name
        info = json.loads(subprocess.check_output(["docker", "inspect", container]))[0]
        env = dict(item.split("=", 1) for item in info["Config"]["Env"] if "=" in item)
        if name == env.get("POSTGRES_DB", env.get("POSTGRES_USER")):
            raise ValueError("The container's installation database is protected; use disposable copies")
        self.user = env["POSTGRES_USER"]

    def command(self, executable, *args):
        return ["docker", "exec", "-i", self.container, executable, "-U", self.user,
                "-d", self.name, *args]

    def sql(self, sql):
        result = subprocess.run(self.command("psql", "-X", "-q", "-At", "-v", "ON_ERROR_STOP=1"),
                                input=sql.encode(), capture_output=True)
        if result.returncode:
            # PostgreSQL errors can contain private row contents.
            raise RuntimeError("Database query failed; no private database diagnostics were printed")
        return result.stdout.decode().strip()

    def schema(self):
        sql = subprocess.check_output(self.command("pg_dump", "--schema-only", "--no-owner", "--no-privileges"))
        return b"\n".join(line for line in sql.splitlines()
                          if not line.startswith((b"\\restrict ", b"\\unrestrict ")))

    def journal(self):
        return json.loads(self.sql("SELECT json_agg(j) FROM (SELECT id,checksum,completed_at IS NOT NULL AS completed,failure FROM citadel_schema_migrations ORDER BY id) j"))

    def columns(self):
        return json.loads(self.sql("SELECT json_object_agg(table_name,cols) FROM (SELECT table_name,json_agg(column_name ORDER BY ordinal_position) AS cols FROM information_schema.columns WHERE table_schema='public' AND table_name<>'citadel_schema_migrations' GROUP BY table_name) c"))


def signature_query(table, columns):
    projection = ",".join(identifier(column) for column in columns)
    return ("SELECT json_build_object('count',count(*),'hash',"
            "md5(COALESCE(string_agg(h,'' ORDER BY h),''))) FROM "
            f"(SELECT md5(row_to_json(q)::text) AS h FROM (SELECT {projection} "
            f"FROM public.{identifier(table)}) q) rows")


VALIDATE_FOREIGN_KEYS = """
DO $validate$
DECLARE c record; equality text; present text; invalid boolean;
BEGIN
  FOR c IN SELECT * FROM pg_constraint WHERE contype='f' AND connamespace='public'::regnamespace LOOP
    IF c.confmatchtype <> 's' THEN RAISE EXCEPTION 'Unsupported foreign key match type'; END IF;
    SELECT string_agg(format('s.%I=t.%I',a.attname,b.attname),' AND ' ORDER BY k.n),
           string_agg(format('s.%I IS NOT NULL',a.attname),' AND ' ORDER BY k.n)
    INTO equality,present
    FROM generate_subscripts(c.conkey,1) k(n)
    JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=c.conkey[k.n]
    JOIN pg_attribute b ON b.attrelid=c.confrelid AND b.attnum=c.confkey[k.n];
    EXECUTE format('SELECT EXISTS(SELECT 1 FROM %s s WHERE %s AND NOT EXISTS(SELECT 1 FROM %s t WHERE %s))',
                   c.conrelid::regclass,present,c.confrelid::regclass,equality) INTO invalid;
    IF invalid THEN RAISE EXCEPTION 'Imported foreign key validation failed: %',c.conname; END IF;
  END LOOP;
END;
$validate$;
"""


def convert(source, target, report):
    if source.container != target.container or source.name == target.name:
        raise ValueError("Use separate disposable databases in the same isolated container")
    for database, checksum in [(source, OLD_BASELINE), (target, NEW_BASELINE)]:
        if database.journal() != [dict(id="0001", checksum=checksum, completed=True, failure=None)]:
            raise ValueError("Database must contain exactly the supported completed baseline")
    columns = source.columns()
    target_columns = target.columns()
    expected = {table: list(names) for table, names in columns.items()}
    expected["alertrulestates"].append("lastobservedat")
    if {k: sorted(v) for k, v in target_columns.items()} != {k: sorted(v) for k, v in expected.items()}:
        raise ValueError("Schema columns differ beyond the supported alert observation timestamp")
    old_schema = source.schema()
    new_schema = target.schema().replace(
        b"    lasttriggeredat timestamp with time zone,\n    lastobservedat timestamp with time zone\n",
        b"    lasttriggeredat timestamp with time zone\n")
    if old_schema != new_schema:
        raise ValueError("Schema objects differ beyond the supported alert observation timestamp")
    checks = " UNION ALL ".join(
        f"SELECT '{table}' AS name,({signature_query(table, names)}) AS value"
        for table, names in sorted(columns.items()))
    signatures = json.loads(source.sql(
        "SET TIME ZONE 'UTC';\nSELECT json_object_agg(name,value) FROM (" + checks + ") signatures"))
    target_journal = target.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j")
    os.umask(0o077)
    report = Path(report)
    if report.exists():
        raise ValueError("Use a new report path; previous conversion evidence is preserved")
    # Private temporary files may contain credentials and resource configuration.
    with tempfile.TemporaryDirectory(prefix="citadel-baseline-import-") as temporary:
        script = Path(temporary) / "import.sql"
        with script.open("wb") as stream:
            stream.write(b"BEGIN;\nSET LOCAL TIME ZONE 'UTC';\nSET LOCAL session_replication_role=replica;\n")
            tables = ",".join("public." + identifier(table) for table in sorted(columns))
            stream.write(f"TRUNCATE {tables} RESTART IDENTITY;\n".encode())
            stream.flush()
            result = subprocess.run(source.command("pg_dump", "--data-only", "--no-owner", "--no-privileges",
                                    "--exclude-table=public." + JOURNAL), stdout=stream, stderr=subprocess.PIPE)
            if result.returncode:
                raise RuntimeError("Private data export failed")
            stream.write(b"\nSET LOCAL session_replication_role=origin;\nSET LOCAL search_path=public,pg_catalog;\n")
            stream.write(b"UPDATE alertrulestates SET lastobservedat=lasttriggeredat;\n")
            stream.write(VALIDATE_FOREIGN_KEYS.encode())
            for table, names in sorted(columns.items()):
                expected_signature = json.dumps(signatures[table]).replace("'", "''")
                query = signature_query(table, names)
                stream.write((f"DO $verify$ BEGIN IF ({query})::jsonb <> '{expected_signature}'::jsonb "
                              f"THEN RAISE EXCEPTION 'Record verification failed: {table}'; END IF; END $verify$;\n").encode())
            stream.write(b"DO $verify$ BEGIN IF EXISTS(SELECT 1 FROM alertrulestates WHERE lastobservedat IS DISTINCT FROM lasttriggeredat) THEN RAISE EXCEPTION 'Observation backfill failed'; END IF; END $verify$;\nCOMMIT;\n")
        with script.open("rb") as stream:
            result = subprocess.run(target.command("psql", "-X", "-q", "-v", "ON_ERROR_STOP=1"),
                                    stdin=stream, capture_output=True)
        if result.returncode:
            diagnostics = report.with_suffix(".error.log")
            diagnostics.write_bytes(result.stderr)
            raise RuntimeError(f"Import transaction rolled back; private diagnostics: {diagnostics}")
    if target.sql("SELECT row_to_json(j) FROM citadel_schema_migrations j") != target_journal:
        raise RuntimeError("Target migration journal changed unexpectedly")
    evidence = dict(sourceBaseline=OLD_BASELINE, targetBaseline=NEW_BASELINE,
                    sourceSchemaSha256=hashlib.sha256(old_schema).hexdigest(),
                    tables=signatures, foreignKeysValidated=True, observationBackfilled=True)
    report.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps(dict(status="converted", tables=len(signatures),
                          rows=sum(value["count"] for value in signatures.values()), report=str(report))))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--source-database", required=True)
    parser.add_argument("--target-database", required=True)
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    convert(Database(args.container, args.source_database), Database(args.container, args.target_database), args.report)


if __name__ == "__main__":
    main()
