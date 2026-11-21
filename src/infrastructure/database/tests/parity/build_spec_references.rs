use super::*;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn build_reference_backfill_and_concurrent_edits_match_recursive_retention() {
    let db = Database::create().await;
    let mut writer = PgConnection::connect(&db.url).await.unwrap();
    let mut observer = PgConnection::connect(&db.url).await.unwrap();
    // The projection also accepts references to runs that do not exist yet.
    sqlx::raw_sql("CREATE TABLE deployments(id uuid PRIMARY KEY,spec jsonb);
        CREATE TABLE stackreleases(LIKE deployments INCLUDING ALL);
        CREATE TABLE buildruns(id uuid PRIMARY KEY,completedat timestamptz,status text);
        CREATE INDEX ix_buildruns_retention ON buildruns(completedat);
        INSERT INTO buildruns(id) SELECT md5(i::text)::uuid FROM generate_series(1,20)i;
        INSERT INTO deployments VALUES(md5('owner')::uuid,jsonb_build_array(md5('1')::uuid::text,upper(md5('2')::uuid::text)));
        INSERT INTO stackreleases VALUES(md5('historical')::uuid,jsonb_build_object('nested',jsonb_build_array(md5('3')::uuid::text)));
        CREATE VIEW original AS SELECT id FROM buildruns r WHERE EXISTS (
            SELECT 1 FROM (SELECT spec FROM deployments UNION ALL SELECT spec FROM stackreleases) owners
            WHERE jsonb_path_exists(spec,'$.** ? (@ == $id)',jsonb_build_object('id',r.id::text)))")
        .execute(&mut writer).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../generated/0011_build_spec_references.sql"
    ))
    .execute(&mut writer)
    .await
    .unwrap();
    async fn check(conn: &mut PgConnection, keys: &[i32]) {
        let (scanned,indexed,expected):(Vec<Uuid>,Vec<Uuid>,Vec<Uuid>)=sqlx::query_as(
            "SELECT ARRAY(SELECT id FROM original ORDER BY id),
                ARRAY(SELECT id FROM buildruns r WHERE EXISTS(SELECT 1 FROM buildspecassignments ref WHERE ref.referenceid=r.id) ORDER BY id),
                ARRAY(SELECT md5(n::text)::uuid FROM unnest($1::int[]) n ORDER BY 1)")
            .bind(keys).fetch_one(conn).await.unwrap();
        assert_eq!(scanned, expected);
        assert_eq!(indexed, scanned);
    }
    check(&mut observer, &[1, 3]).await;
    for commit in [false, true] {
        let mut tx = writer.begin().await.unwrap();
        sqlx::raw_sql("UPDATE deployments SET spec=jsonb_build_object('nested',jsonb_build_array(md5('7')::uuid::text,md5('7')::uuid::text));
            DELETE FROM stackreleases").execute(&mut *tx).await.unwrap();
        check(&mut tx, &[7]).await;
        check(&mut observer, &[1, 3]).await;
        if commit {
            tx.commit().await.unwrap();
        } else {
            tx.rollback().await.unwrap();
        }
    }
    check(&mut observer, &[7]).await;
    drop(observer);
    drop(writer);
    db.close().await;
}
