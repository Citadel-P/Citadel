use super::*;

// Additional backup/recovery regression: database URL components must reach
// libpq decoded exactly once, without putting credentials in process arguments.
#[test]
fn postgres_tools_decode_url_components_without_treating_plus_as_space() {
    let url = "postgres://citadel%40ops:p%40ss%3Aword%2F%25%2B+@database:5432/citadel%20backup?sslmode=require";
    for (arguments, environment) in [
        pg_dump_request(url, Path::new("citadel.dump")).unwrap(),
        pg_restore_request(url, Path::new("citadel.dump")).unwrap(),
    ] {
        for (key, expected) in [
            ("PGUSER", "citadel@ops"),
            ("PGPASSWORD", "p@ss:word/%++"),
            ("PGDATABASE", "citadel backup"),
        ] {
            let value = environment.iter().find(|(name, _)| name == key).unwrap();
            assert_eq!(value.1, OsString::from(expected), "{key}");
        }
        assert!(
            !arguments
                .iter()
                .any(|a| a.to_string_lossy().contains("p@ss"))
        );
    }
    assert_eq!(postgres_database_name(url).unwrap(), "citadel backup");
}

#[test]
fn postgres_tools_reject_decoded_nul_before_spawning() {
    for url in [
        "postgres://user:pass%00word@db/data",
        "postgres://us%00er:pass@db/data",
        "postgres://user:pass@db/da%00ta",
    ] {
        assert!(pg_dump_request(url, Path::new("dump")).is_err());
        assert!(pg_restore_request(url, Path::new("dump")).is_err());
    }
}

#[test]
fn restore_database_name_cannot_override_libpq_connection_parameters() {
    let (arguments, _) = pg_restore_request(
        "postgres://user:pass@database/db%27%20host%3Dother%20%5Cname",
        Path::new("dump"),
    )
    .unwrap();
    let index = arguments.iter().position(|arg| arg == "--dbname").unwrap();
    assert_eq!(
        arguments[index + 1],
        OsString::from("dbname='db\\' host=other \\\\name'")
    );
}
