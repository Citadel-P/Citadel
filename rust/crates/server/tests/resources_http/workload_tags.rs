use super::*;

// ResourceTagIntegrationTests: replacement must persist atomically and the
// resource's own permission controls both reads and writes.
pub(super) async fn verify(
    app: &Router,
    pool: &sqlx::PgPool,
    admin: &ActorPrincipal,
    tags: &[Value],
) {
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES ($1,$3,'Local',1,0,0,$2,0,'{}','Online',0)")
        .bind(platform).bind(format!("tags-{platform}")).bind(format!("unix:///tags-{platform}.sock")).execute(pool).await.unwrap();
    for (path, resource, insert) in [
        (
            "deployments",
            ResourceType::Deployment,
            "INSERT INTO deployments(id,name,platformid,createdbyactorid,spec,status) VALUES ($1,$2,$3,$4,'{}','Created')",
        ),
        (
            "stacks",
            ResourceType::Stack,
            "INSERT INTO stacks(id,name,createdbyactorid,driftpolicy,stacksource,stackupdatestate) SELECT $1,$2,$4,'{}','WebEditor','{}' WHERE $3::uuid IS NOT NULL",
        ),
        (
            "swarmServices",
            ResourceType::SwarmService,
            "INSERT INTO swarmservices(id,name,platformid,createdbyactorid,spec,desiredspechash,dockername,health,synchronizationstate,updatedat) VALUES ($1,$2,$3,$4,'{}','hash',$2,'Created','Synchronized',CURRENT_TIMESTAMP)",
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query(insert)
            .bind(id)
            .bind(format!("tags-{id}"))
            .bind(platform)
            .bind(admin.actor_id.value())
            .execute(pool)
            .await
            .unwrap();
        let uri = format!("/api/v1/{path}/{id}/tags");
        let mut reader = admin.clone();
        reader.actor_id = ActorId::new(Uuid::now_v7());
        reader.roles.clear();
        sqlx::query("INSERT INTO actors(id,isenabled,type) VALUES($1,TRUE,'User')")
            .bind(reader.actor_id.value())
            .execute(pool)
            .await
            .unwrap();
        for method in [Method::GET, Method::PUT] {
            let body = (method == Method::PUT).then(|| json!({"tagIds":[tags[0]["id"]]}));
            assert_eq!(
                request(app, method.clone(), &uri, None, body.clone())
                    .await
                    .status(),
                StatusCode::UNAUTHORIZED
            );
            assert_eq!(
                request(app, method, &uri, Some(reader.clone()), body)
                    .await
                    .status(),
                StatusCode::FORBIDDEN
            );
        }
        for tag in tags {
            let response = request(
                app,
                Method::PUT,
                &uri,
                Some(admin.clone()),
                Some(json!({"tagIds":[tag["id"]]})),
            )
            .await;
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "{path}: {}",
                response_json(response).await
            );
            let stored: Vec<Uuid> =
                sqlx::query_scalar("SELECT tagid FROM resourcetags WHERE resourceid=$1")
                    .bind(id)
                    .fetch_all(pool)
                    .await
                    .unwrap();
            assert_eq!(
                stored,
                vec![Uuid::parse_str(tag["id"].as_str().unwrap()).unwrap()]
            );
        }
        assert_eq!(
            request(
                app,
                Method::PUT,
                &uri,
                Some(admin.clone()),
                Some(json!({"tagIds":[Uuid::now_v7()]}))
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        let after =
            response_json(request(app, Method::GET, &uri, Some(admin.clone()), None).await).await;
        assert_eq!(after["tags"][0]["id"], tags[1]["id"]);
        sqlx::query("INSERT INTO resourceaccesses(id,actorid,permissionlevel,resourceid,resourcetype,specificpermissions) VALUES($1,$2,1,$3,$4,0)")
            .bind(Uuid::now_v7()).bind(reader.actor_id.value()).bind(id).bind(resource as i32).execute(pool).await.unwrap();
        assert_eq!(
            request(app, Method::GET, &uri, Some(reader.clone()), None)
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            request(
                app,
                Method::PUT,
                &uri,
                Some(reader),
                Some(json!({"tagIds":[]}))
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let response = request(
            app,
            Method::PUT,
            &uri,
            Some(admin.clone()),
            Some(json!({"tagIds":[]})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["tags"], json!([]));
        assert_eq!(
            request(
                app,
                Method::GET,
                &format!("/api/v1/{path}/{}/tags", Uuid::now_v7()),
                Some(admin.clone()),
                None
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
    }
}
