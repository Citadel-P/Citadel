use sqlx::PgPool;

#[tokio::test]
#[ignore = "requires empty disposable CITADEL_UPGRADE_DATABASE_URL; rehearses baseline restoration"]
async fn baseline_restoration_backfills_secret_grants_without_exposing_unbound_secrets() {
    let pool = PgPool::connect(&std::env::var("CITADEL_UPGRADE_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../generated/0001_initial.sql"))
        .execute(&pool)
        .await
        .unwrap();
    // Synthetic identities, ciphertext and references only; no execution is involved.
    sqlx::raw_sql(r#"
        INSERT INTO actors(id,type) VALUES('10000000-0000-0000-0000-000000000001','User');
        INSERT INTO secretdefinitions(id,name,providertype)
            SELECT ('20000000-0000-0000-0000-'||lpad(n::text,12,'0'))::uuid,'fixture-'||n,
                CASE WHEN n=7 THEN 'VaultCompatibleKvV2' ELSE 'InternalEncrypted' END FROM generate_series(1,7) n;
        INSERT INTO internalsecretvalues(secretid,encryptedvalue)
            SELECT id,'synthetic-ciphertext' FROM secretdefinitions WHERE providertype='InternalEncrypted';
        INSERT INTO resourcebindings(id,kind,name,resourceid,scope,secretid) VALUES
            (gen_random_uuid(),'Secret','stack-token','30000000-0000-0000-0000-000000000001','Stack','20000000-0000-0000-0000-000000000001'),
            (gen_random_uuid(),'Secret','global-token',NULL,'Global','20000000-0000-0000-0000-000000000002');
        INSERT INTO backuprepositories(id,createdbyactorid,name,normalizedname,passwordsecretid,spec,status,type) VALUES
            ('40000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000001','backup','BACKUP','20000000-0000-0000-0000-000000000003',
             '{"accessKeySecretId":"20000000-0000-0000-0000-000000000004","secretKeySecretId":"20000000-0000-0000-0000-000000000005"}','Ready','S3');
        INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES
            ('50000000-0000-0000-0000-000000000001','http://fixture.test','Agent',0,0,0,'fixture',0,'{}','Offline',0);
        INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,url) VALUES
            ('60000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000001','main','fixture','Unknown','https://fixture.test/repo');
        INSERT INTO buildprojects(id,branch,createdbyactorid,gitrepositoryid,imagerepository,name,normalizedname,platformid,buildsecrets) VALUES
            ('70000000-0000-0000-0000-000000000001','main','10000000-0000-0000-0000-000000000001','60000000-0000-0000-0000-000000000001','fixture','fixture','FIXTURE',
            '50000000-0000-0000-0000-000000000001','[{"secretId":"20000000-0000-0000-0000-000000000005","name":"token"}]');
    "#).execute(&pool).await.unwrap();
    let baseline = include_str!("../generated/0001_initial.sql");
    let backfill = baseline
        .split_once("-- BEGIN RESTORE SECRET USE GRANTS\n")
        .unwrap()
        .1
        .split_once("-- END RESTORE SECRET USE GRANTS")
        .unwrap()
        .0;
    sqlx::raw_sql(backfill).execute(&pool).await.unwrap();
    // A resumed restore must not duplicate or broaden the derived grants.
    sqlx::raw_sql(backfill).execute(&pool).await.unwrap();
    let grants: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT secretid::text,scope,resourceid::text FROM secretusegrants ORDER BY secretid,scope",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let expected = [
        (1, "Stack", "30000000-0000-0000-0000-000000000001"),
        (2, "Global", "00000000-0000-0000-0000-000000000000"),
        (
            3,
            "BackupRepository",
            "40000000-0000-0000-0000-000000000001",
        ),
        (
            4,
            "BackupRepository",
            "40000000-0000-0000-0000-000000000001",
        ),
        (
            5,
            "BackupRepository",
            "40000000-0000-0000-0000-000000000001",
        ),
        (5, "Build", "70000000-0000-0000-0000-000000000001"),
        (7, "Global", "00000000-0000-0000-0000-000000000000"),
    ]
    .into_iter()
    .map(|(n, scope, id)| {
        (
            format!("20000000-0000-0000-0000-{n:012}"),
            scope.to_owned(),
            id.to_owned(),
        )
    })
    .collect::<Vec<_>>();
    assert_eq!(grants, expected);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM internalsecretvalues WHERE encryptedvalue='synthetic-ciphertext'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        6
    );
}
