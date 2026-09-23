use super::*;
use citadel_adapters::persistence::postgres::discovery::search::PostgresGlobalSearchStore;
use citadel_primitives::ResourceType;

// Ports all GlobalSearchTests scenarios: ranking, literal LIKE characters,
// per-resource grants, parent redaction, Swarm Services, role/team grants and validation.
#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn global_search_preserves_ranking_permissions_parent_redaction_and_bounds() {
    let mut f = fixture().await;
    f.app = citadel_server::api::routes::search::router(Arc::new(PostgresGlobalSearchStore::new(
        f.pool.clone(),
    )));
    let q = format!("search{}", Uuid::now_v7().simple());
    let mut platforms = Vec::new();
    for name in [
        q.clone(),
        format!("{q}-east"),
        format!("east-{q}"),
        format!("literal%{q}"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO platforms(id,name,address,connectortype,platformdescriptor,status,cpucount,imagecount,memtotal,networkcount,volumecount) VALUES($1,$2,$3,'Local','{\"$type\":\"Docker\"}','Online',0,0,0,0,0)")
            .bind(id).bind(name).bind(format!("unix:///{id}" )).execute(&f.pool).await.unwrap();
        platforms.push(id);
    }
    let uri = format!("/api/v1/search?q={q}&types=Platform");
    let response = send(&f, &uri, Some(f.administrator.clone())).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "{}",
        json_body(response).await
    );
    let results = json_body(send(&f, &uri, Some(f.administrator.clone())).await).await;
    assert_eq!(results["groups"][0]["items"].as_array().unwrap().len(), 4);
    assert_eq!(
        results["groups"][0]["items"][0]["id"],
        platforms[0].to_string()
    );
    assert_eq!(
        results["groups"][0]["items"][1]["id"],
        platforms[1].to_string()
    );
    assert_eq!(
        results["groups"][0]["items"][0]["status"]["tone"],
        "Positive"
    );
    let literal = json_body(
        send(
            &f,
            &format!("/api/v1/search?q=%25{q}&types=Platform"),
            Some(f.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(literal["groups"][0]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        literal["groups"][0]["items"][0]["id"],
        platforms[3].to_string()
    );
    assert_eq!(
        send(&f, &uri, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    for query in [
        "q=a",
        "q=prod&types=Unknown",
        "q=prod&limitPerType=11",
        "q=prod&limitPerType=bad",
        "q=prod&types=,",
    ] {
        assert_eq!(
            send(
                &f,
                &format!("/api/v1/search?{query}"),
                Some(f.administrator.clone())
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let deployment = Uuid::now_v7();
    let service = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES($1,$2,$3,$4,'{}','Created')")
        .bind(deployment).bind(format!("{q}-api")).bind(platforms[0]).bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO swarmservices(id,name,dockername,platformid,createdbyactorid,spec,desiredspechash,health,synchronizationstate,updatedat) VALUES($1,$2,$2,$3,$4,'{}','fixture','Created','Synchronized',CURRENT_TIMESTAMP)")
        .bind(service).bind(format!("{q}-cache")).bind(platforms[0]).bind(f.administrator.actor_id.value()).execute(&f.pool).await.unwrap();
    let mut reader = f.administrator.clone();
    reader.roles.clear();
    reader.actor_id = ActorId::new(Uuid::now_v7());
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'User')")
        .bind(reader.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    for (kind, id) in [
        (ResourceType::Deployment, deployment),
        (ResourceType::SwarmService, service),
    ] {
        grant(&f, reader.actor_id.value(), kind, id).await;
    }
    let found =
        json_body(send(&f, &format!("/api/v1/search?q={q}"), Some(reader.clone())).await).await;
    assert_eq!(found["groups"].as_array().unwrap().len(), 1);
    let item = &found["groups"][0]["items"][0];
    assert_eq!(item["id"], deployment.to_string());
    assert!(item.get("parent").is_none());
    assert!(item.get("secondaryText").is_none());
    grant(
        &f,
        reader.actor_id.value(),
        ResourceType::Platform,
        platforms[0],
    )
    .await;
    let found = json_body(
        send(
            &f,
            &format!("/api/v1/search?q={q}&types=Deployment,SwarmService"),
            Some(reader.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(found["groups"].as_array().unwrap().len(), 2);
    assert_eq!(
        found["groups"][0]["items"][0]["parent"]["id"],
        platforms[0].to_string()
    );
    assert_eq!(found["groups"][1]["items"][0]["id"], service.to_string());
    assert_eq!(found["groups"][1]["items"][0]["status"]["label"], "Created");
    // Global grants through either a direct role or a Team must produce identical visibility.
    let role = Uuid::now_v7();
    sqlx::query("INSERT INTO roles(id,name,roletype) VALUES($1,$2,'Custom')")
        .bind(role)
        .bind(role.to_string())
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO permissions(id,roleid,resourcetype,permissionlevel,specificpermissions) VALUES($1,$2,0,1,0)").bind(Uuid::now_v7()).bind(role).execute(&f.pool).await.unwrap();
    sqlx::query("INSERT INTO actorroles(actorid,roleid) VALUES($1,$2)")
        .bind(reader.actor_id.value())
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    let direct = json_body(send(&f, &uri, Some(reader.clone())).await).await;
    assert_eq!(direct["groups"][0]["items"].as_array().unwrap().len(), 4);
    let team_actor = Uuid::now_v7();
    let team = Uuid::now_v7();
    sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,true,'Team')")
        .bind(team_actor)
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO teams(id,actorid,name) VALUES($1,$2,$3)")
        .bind(team)
        .bind(team_actor)
        .bind(team.to_string())
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO actorteammemberships(teamid,memberactorid) VALUES($1,$2)")
        .bind(team)
        .bind(reader.actor_id.value())
        .execute(&f.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE actorroles SET actorid=$2 WHERE actorid=$1 AND roleid=$3")
        .bind(reader.actor_id.value())
        .bind(team_actor)
        .bind(role)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(json_body(send(&f, &uri, Some(reader)).await).await, direct);
    let limited = json_body(
        send(
            &f,
            &format!("{uri}&limitPerType=1"),
            Some(f.administrator.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(limited["groups"][0]["items"].as_array().unwrap().len(), 1);
    f.docker_server.abort();
    let _ = tokio::fs::remove_file(f.docker_socket).await;
}

async fn grant(f: &Fixture, actor: Uuid, kind: ResourceType, id: Uuid) {
    sqlx::query("INSERT INTO resourceaccesses(id,actorid,resourcetype,resourceid,permissionlevel,specificpermissions) VALUES($1,$2,$3,$4,1,0)")
        .bind(Uuid::now_v7()).bind(actor).bind(kind as i32).bind(id).execute(&f.pool).await.unwrap();
}
