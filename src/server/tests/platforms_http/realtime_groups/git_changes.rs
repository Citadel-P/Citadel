use super::scoped_fixture::*;
use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn git_catalog_patches_only_the_changed_owner_and_recovers_the_full_list() {
    let f = fixture().await;
    let [a, b] = std::array::from_fn(|_| Uuid::now_v7());
    for id in [a, b] {
        sqlx::query("INSERT INTO gitrepositories(id,createdbyactorid,defaultbranch,name,status,syncmode,url) VALUES($1,$2,'main',$1::text,'Healthy','Manual','file:///unused')")
            .bind(id).bind(SYSTEM_ACTOR_ID).execute(&f.pool).await.unwrap();
    }
    let (hub, reads, address, cancellation, server) = serve(&f).await;
    let mut socket = connect(&f, &f.administrator, address).await;
    join(&mut socket, "git-repositories", 2).await;
    join(&mut socket, &format!("git-repo:{b}"), 1).await;
    let notify =
        citadel_server::realtime::owner_change_callback(Some(hub.clone()), "GitRepository");
    for repeat in 0..2 {
        notify(Some(a));
        barrier(&hub);
        let update = read_until_barrier(&mut socket).await;
        assert_eq!(reads.take(), ["git-repositories"]);
        assert_eq!(update.len(), 1, "notification {repeat}");
        assert_eq!(update[0]["arguments"][0]["id"], json!(a));
    }
    hub.publish_resource_change("GitRepository", a, "webhookReceived");
    barrier(&hub);
    assert_eq!(read_until_barrier(&mut socket).await.len(), 1);
    assert_eq!(reads.take(), ["git-repositories"]);
    notify(None);
    barrier(&hub);
    assert_eq!(read_until_barrier(&mut socket).await.len(), 3);
    sqlx::query("DELETE FROM gitrepositories WHERE id=$1")
        .bind(a)
        .execute(&f.pool)
        .await
        .unwrap();
    notify(Some(a));
    barrier(&hub);
    let update = read_until_barrier(&mut socket).await;
    assert_eq!(update.len(), 1);
    assert_eq!(update[0]["arguments"][1], "delete");
    assert_eq!(update[0]["arguments"][0]["id"], json!(a));
    socket.close(None).await.unwrap();
    cancellation.cancel();
    server.await.unwrap();
    cleanup(f).await;
}
