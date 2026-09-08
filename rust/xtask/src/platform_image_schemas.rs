use serde_json::{Map, Value, json};

pub fn add(schemas: &mut Map<String, Value>) {
    let s = json!({"type":"string"});
    let n = json!({"type":["string","null"]});
    let b = json!({"type":"boolean"});
    let i = json!({"type":"integer","format":"int64"});
    let uuid = json!({"type":"string","format":"uuid"});
    let array = json!({"type":"array","items":{"type":"string"}});
    schemas.insert("PlatformInput".into(),json!({"type":"object","properties":{"name":s,"address":n,"description":n,"type":{"type":"string","enum":["Docker","DockerSwarm","Kubernetes"]},"connectorType":{"type":"string","enum":["Unknown","Local","Agent","EdgeAgent"]},"pruneHistoricalSwarmTaskContainers":b}}));
    let resource = json!({"type":"string","enum":["All","Volume","Network","Image","Build"]});
    schemas.insert(
        "PrunePlatformInput".into(),
        json!({"type":"object","required":["resource"],"properties":{"resource":resource}}),
    );
    schemas.insert("PrunePlatformView".into(),json!({"type":"object","required":["resource","spaceReclaimed","volumesDeleted","networksDeleted","imagesDeleted","buildCacheDeleted"],"properties":{"resource":resource,"spaceReclaimed":i,"volumesDeleted":array,"networksDeleted":array,"imagesDeleted":array,"buildCacheDeleted":array}}));
    schemas.insert("PullImageInput".into(),json!({"type":"object","required":["platformId","registryId","imageTag"],"properties":{"platformId":uuid,"registryId":uuid,"imageTag":s}}));
    schemas.insert("PullImageStreamItem".into(),json!({"type":"object","properties":{"id":n,"from":n,"stream":n,"status":n,"errorMessage":n,"progressMessage":n,"dockerImageId":n,"digest":n,"progress":{"type":["object","null"],"properties":{"units":n,"current":i,"total":i,"start":i}},"error":{"type":["object","null"],"properties":{"code":i,"message":n}}}}));
    schemas.insert("DockerHubRepositoryInfo".into(),json!({"type":"object","properties":{"name":n,"namespace":n,"lastUpdated":n,"isPrivate":b,"isTrusted":b,"isAutomated":b,"pullCount":i}}));
    schemas.insert("IImageRepository".into(),json!({"oneOf":[{"type":"object","required":["$type","name"],"properties":{"$type":{"const":"DockerHub"},"name":s,"namespace":n,"lastUpdated":n,"isPrivate":b,"pullCount":i}},{"type":"object","required":["$type","id","name"],"properties":{"$type":{"const":"GitHub"},"id":s,"name":s,"createdAt":n,"updatedAt":n,"url":n,"htmlUrl":n}}]}));
    schemas.insert("GitHubCrPackageVersion".into(),json!({"type":"object","required":["id","name"],"properties":{"id":i,"name":s,"url":n,"htmlUrl":n,"createdAt":n,"updatedAt":n,"packageHtmlUrl":n,"metadata":{"type":["object","null"],"properties":{"container":{"type":["object","null"],"properties":{"tags":array}}}}}}));
    schemas.insert("DockerHubTagView".into(),json!({"type":"object","properties":{"id":i,"name":s,"lastUpdated":n,"fullSize":i,"status":{"type":"string","enum":["Active","Inactive"]},"lastPulled":n,"image":{"type":["object","null"],"properties":{"architecture":s,"digest":s,"os":s,"size":i,"status":{"type":"string","enum":["Active","Inactive"]},"lastPulled":n}}}}));
}
