use super::*;
use citadel_adapters::persistence::postgres::tags::PostgresTagRepository;
use citadel_server::api::routes::tags::{TagsHttpState, router};

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_tags_survive_fresh_detail_list_and_realtime_reads() {
    let mut fixture = fixture().await;
    fixture.app = fixture.app.clone().merge(router(TagsHttpState {
        identity: fixture.lookup_state.platforms.identity.clone(),
        tags: Arc::new(PostgresTagRepository::new(fixture.pool.clone())),
        realtime: None,
    }));
    let detail_uri = format!("/api/v1/platforms/{}", fixture.platform_id);
    let tags_uri = format!("{detail_uri}/tags");

    // Clear the fixture's tag, add it through the real mutation, then remove it.
    for tag_ids in [vec![], vec![fixture.tag_id], vec![]] {
        let saved = json_body(
            send_json(
                &fixture,
                Method::PUT,
                &tags_uri,
                fixture.administrator.clone(),
                json!({"tagIds": tag_ids}),
            )
            .await,
        )
        .await;
        assert_eq!(saved["tags"].as_array().unwrap().len(), tag_ids.len());
        if !tag_ids.is_empty() {
            assert_eq!(saved["tags"][0]["id"], json!(fixture.tag_id));
            assert_eq!(saved["tags"][0]["color"], "blue");
            assert!(!saved["tags"][0]["name"].as_str().unwrap().is_empty());
        }

        let detail =
            json_body(send(&fixture, &detail_uri, Some(fixture.administrator.clone())).await).await;
        assert_eq!(detail["tags"], saved["tags"]);
        let list = json_body(
            send(
                &fixture,
                "/api/v1/platforms",
                Some(fixture.administrator.clone()),
            )
            .await,
        )
        .await;
        let platform = list["platforms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|platform| platform["id"] == json!(fixture.platform_id))
            .unwrap();
        assert_eq!(platform["tags"], saved["tags"]);
        let realtime = fixture
            .realtime
            .authorize_platform(&fixture.administrator, fixture.platform_id)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(realtime).unwrap()["tags"],
            saved["tags"]
        );
    }
    fixture.docker_server.abort();
}

#[tokio::test]
#[ignore = "requires CITADEL_PHASE4_DATABASE_URL"]
async fn platform_tag_filters_accept_names_and_ids_and_require_all_selected_tags() {
    let fixture = fixture().await;
    sqlx::query("UPDATE tags SET name='Prod & Europe', normalizedname='PROD & EUROPE' WHERE id=$1")
        .bind(fixture.tag_id)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let reader = ActorPrincipal {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(fixture.actor_id),
        name: "reader".into(),
        principal_type: AuthenticatedPrincipalType::User,
        credential_id: None,
        roles: vec![],
    };
    for query in [
        "tags=Prod%20%26%20Europe".to_owned(),
        "TaGs=prod+%26+europe&TAGS=PROD+%26+EUROPE&tags=%20".to_owned(),
        format!("tags={}", fixture.tag_id),
    ] {
        let response = send(
            &fixture,
            &format!("/api/v1/platforms?{query}"),
            Some(reader.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK, "{query}");
        let body = json_body(response).await;
        assert_eq!(body["platforms"].as_array().unwrap().len(), 1, "{query}");
        assert_eq!(body["platforms"][0]["id"], json!(fixture.platform_id));
    }

    for query in ["tags=missing", "tags=Prod+%26+Europe&tags=blue"] {
        let response = send(
            &fixture,
            &format!("/api/v1/platforms?{query}"),
            Some(reader.clone()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            json_body(response).await["platforms"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let second_tag = Uuid::now_v7();
    sqlx::query("INSERT INTO tags(id,color,createdbyactorid,name,normalizedname) VALUES($1,'blue',$2,'blue','BLUE')")
        .bind(second_tag).bind(fixture.actor_id).execute(&fixture.pool).await.unwrap();
    sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('Platform',$1,$2,$3)")
        .bind(fixture.platform_id).bind(second_tag).bind(fixture.actor_id).execute(&fixture.pool).await.unwrap();
    let uri = "/api/v1/platforms?tags=Prod+%26+Europe&tags=blue";
    let matching = json_body(send(&fixture, uri, Some(reader.clone())).await).await;
    assert_eq!(matching["platforms"].as_array().unwrap().len(), 1);

    let denied = ActorPrincipal {
        actor_id: ActorId::new(Uuid::now_v7()),
        ..reader
    };
    let unauthorized = json_body(send(&fixture, uri, Some(denied)).await).await;
    assert!(unauthorized["platforms"].as_array().unwrap().is_empty());
    assert_eq!(
        send(&fixture, uri, None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    fixture.docker_server.abort();
}
