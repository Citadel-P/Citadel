use super::{nullable_date_time, nullable_string, string, string_array, uuid};
use serde_json::{Map, Value, json};

pub(super) fn add(schemas: &mut Map<String, Value>) {
    let labels = json!({"type":"object","additionalProperties":{"type":"string"}});
    let version = json!({"type":"integer","format":"int64","minimum":0});
    let availability = json!({"type":"string","enum":["Active","Pause","Drain"]});
    schemas.insert(
        "UpdateSwarmNodeInput".into(),
        json!({"type":"object","required":["versionIndex","availability"],"properties":{
        "versionIndex":version,"availability":availability,"labels":labels}}),
    );
    schemas.insert("UpdateSwarmNodesAvailabilityInput".into(),json!({"type":"object","required":["nodes","availability"],"properties":{
        "availability":availability,"nodes":{"type":"array","minItems":1,"maxItems":100,"items":{"type":"object","required":["nodeId","versionIndex"],"properties":{"nodeId":string(),"versionIndex":version}}}}}));
    for name in ["CreateSwarmSecretInput", "CreateSwarmConfigInput"] {
        schemas.insert(name.into(),json!({"type":"object","required":["name","data"],"properties":{"name":string(),"data":string(),"labels":labels}}));
    }
    schemas.insert("UpdateSwarmResourceLabelsInput".into(),json!({"type":"object","required":["versionIndex","labels"],"properties":{"versionIndex":version,"labels":labels}}));
    schemas.insert(
        "DeleteSwarmResourcesInput".into(),
        json!({"type":"object","required":["ids"],"properties":{"ids":string_array()}}),
    );
    schemas.insert(
        "SwarmConfigDataView".into(),
        json!({"type":"object","required":["content"],"properties":{"content":string()}}),
    );
    schemas.insert("SwarmNodeInspectView".into(),json!({"type":"object","required":["id","versionIndex","hostname","role","isLeader","reachability","status","statusMessage","availability","engineVersion","operatingSystem","architecture","address","labels","runningTaskCount","desiredTaskCount","createdAt","updatedAt"],"properties":{
        "id":string(),"versionIndex":version,"hostname":string(),"role":string(),"availability":string(),"status":string(),"statusMessage":nullable_string(),
        "address":string(),"reachability":string(),"isLeader":{"type":"boolean"},
        "operatingSystem":string(),"architecture":string(),"engineVersion":string(),
        "runningTaskCount":{"type":"integer"},"desiredTaskCount":{"type":"integer"},"labels":labels,"createdAt":nullable_date_time(),"updatedAt":nullable_date_time()}}));
    schemas.insert("SwarmServiceInspectView".into(),json!({"type":"object","required":["id","versionIndex","name","mode","image","runningTaskCount","desiredTaskCount","updateState","updateMessage","ports","networkIds","secretIds","configIds","labels","createdAt","updatedAt"],"properties":{
        "id":string(),"versionIndex":version,"name":string(),"mode":string(),"image":string(),"runningTaskCount":{"type":"integer"},"desiredTaskCount":{"type":"integer"},
        "updateState":string(),"updateMessage":nullable_string(),"ports":string_array(),"networkIds":string_array(),"secretIds":string_array(),"configIds":string_array(),
        "labels":labels,"createdAt":nullable_date_time(),"updatedAt":nullable_date_time()}}));
    schemas.insert("SwarmServiceDuplicateDraftView".into(),json!({"type":"object","required":["draft","warnings"],"properties":{
        "draft":{"$ref":"#/components/schemas/CreateSwarmServiceInput"},
        "warnings":{"type":"array","items":{"type":"object","required":["code","message"],"properties":{"code":string(),"message":string()}}}}}));
    schemas.insert("SwarmServiceAdoptionDraftView".into(),json!({"type":"object","required":["source","draft","issues","previewFingerprint"],"properties":{
        "source":{"type":"object","required":["dockerServiceId","name","platformId","platformName"],"properties":{"dockerServiceId":string(),"name":string(),"platformId":uuid(),"platformName":string()}},
        "draft":{"$ref":"#/components/schemas/CreateSwarmServiceInput"},"previewFingerprint":string(),
        "issues":{"type":"array","items":{"type":"object","required":["code","message"],"properties":{"code":string(),"message":string()}}}}}));
    schemas.insert("AdoptSwarmServiceInput".into(),json!({"type":"object","required":["name","spec","previewFingerprint"],"properties":{
        "name":string(),"description":nullable_string(),"spec":{"$ref":"#/components/schemas/SwarmServiceSpec"},"previewFingerprint":{"type":"string","minLength":64,"maxLength":64},
        "tagIds":{"type":"array","items":uuid()}}}));
}
