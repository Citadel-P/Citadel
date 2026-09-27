//! Docker/Agent inventory JSON projected into the existing public resource fields.
//! Move selected fields; never rename user-owned dictionary keys recursively.
use serde_json::{Value, json};

fn project(value: Value, fields: &[(&str, &str)]) -> Value {
    let Value::Object(mut source) = value else {
        return Value::Null;
    };
    Value::Object(
        fields
            .iter()
            .map(|(docker, public)| {
                (
                    (*public).into(),
                    source
                        .remove(*docker)
                        .or_else(|| source.remove(*public))
                        .unwrap_or_default(),
                )
            })
            .collect(),
    )
}
fn array(value: Value, map: fn(Value) -> Value) -> Value {
    Value::Array(match value {
        Value::Array(values) => values.into_iter().map(map).collect(),
        _ => Vec::new(),
    })
}

pub(crate) fn ipam(value: Value) -> Value {
    let mut value = project(
        value,
        &[
            ("Driver", "driver"),
            ("Config", "config"),
            ("Options", "options"),
        ],
    );
    if value.is_null() {
        return value;
    }
    value["config"] = array(value["config"].take(), |v| {
        project(
            v,
            &[
                ("Subnet", "subnet"),
                ("IPRange", "ipRange"),
                ("Gateway", "gateway"),
            ],
        )
    });
    if value["options"].is_null() {
        value["options"] = json!({});
    }
    value
}
pub(crate) fn network_container(value: Value) -> Value {
    project(
        value,
        &[
            ("Name", "name"),
            ("EndpointID", "endpointId"),
            ("MacAddress", "macAddress"),
            ("IPv4Address", "ipV4Address"),
            ("IPv6Address", "ipv6Address"),
        ],
    )
}
pub(crate) fn peer(value: Value) -> Value {
    project(value, &[("Name", "name"), ("IP", "ip")])
}

pub(crate) fn cluster_volume(value: Value) -> Value {
    let mut v = project(
        value,
        &[
            ("ID", "id"),
            ("Version", "version"),
            ("CreatedAt", "createdAt"),
            ("UpdatedAt", "updatedAt"),
            ("Spec", "spec"),
            ("Info", "info"),
            ("PublishStatus", "publishStatus"),
        ],
    );
    if v.is_null() {
        return v;
    }
    v["version"] = project(v["version"].take(), &[("Index", "index")]);
    v["spec"] = project(
        v["spec"].take(),
        &[("Group", "group"), ("AccessMode", "accessMode")],
    );
    if !v["spec"].is_null() {
        let mut a = project(
            v["spec"]["accessMode"].take(),
            &[
                ("Scope", "scope"),
                ("Sharing", "sharing"),
                ("Secrets", "secrets"),
                ("CapacityRange", "capacityRange"),
                ("Availability", "availability"),
            ],
        );
        if !a.is_null() {
            for (key, names) in [
                ("scope", &["Single", "Multi"][..]),
                ("sharing", &["None", "ReadOnly", "OneWriter", "All"][..]),
            ] {
                if let Some(name) = a[key]
                    .as_str()
                    .and_then(|s| names.iter().find(|name| name.eq_ignore_ascii_case(s)))
                {
                    a[key] = json!(name);
                }
            }
            a["secrets"] = array(a["secrets"].take(), |s| {
                project(s, &[("Key", "key"), ("Secret", "secret")])
            });
            a["capacityRange"] = project(
                a["capacityRange"].take(),
                &[
                    ("RequiredBytes", "requiredBytes"),
                    ("LimitBytes", "limitBytes"),
                ],
            );
        }
        v["spec"]["accessMode"] = a;
    }
    v["info"] = project(
        v["info"].take(),
        &[
            ("CapacityBytes", "capacityBytes"),
            ("VolumeContext", "volumeContext"),
            ("VolumeID", "volumeID"),
            ("AccessibleTopology", "accessibleTopology"),
        ],
    );
    if !v["info"].is_null() {
        v["info"]["accessibleTopology"] = array(v["info"]["accessibleTopology"].take(), |t| {
            // Raw Docker topologies are dictionaries; public topologies wrap them.
            if t.get("labels").is_some_and(Value::is_object) {
                t
            } else {
                json!({"labels": t})
            }
        });
    }
    v["publishStatus"] = array(v["publishStatus"].take(), |s| {
        project(
            s,
            &[
                ("NodeID", "nodeID"),
                ("State", "state"),
                ("PublishContext", "publishContext"),
            ],
        )
    });
    v
}

pub(crate) fn system_network(name: &str, ingress: bool) -> bool {
    ingress
        || ["bridge", "host", "none", "nat", "docker_gwbridge"]
            .iter()
            .any(|s| s.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_flags_use_existing_api_names() {
        let view = super::super::views::NetworkView {
            enable_ipv4: true,
            enable_ipv6: true,
            ..Default::default()
        };
        let value = serde_json::to_value(view).unwrap();
        assert_eq!(value["enableIPv4"], true);
        assert_eq!(value["enableIPv6"], true);
        assert!(value.get("enableIpv4").is_none());
    }
    #[test]
    fn network_fields_match_the_frontend_and_preserve_dictionary_keys() {
        let value = ipam(
            json!({"Driver":"default", "Options":{"Mixed.Option":"yes"}, "Config":[{"Subnet":"10.0.0.0/24", "IPRange":"10.0.0.0/25", "Gateway":"10.0.0.1", "AuxiliaryAddresses":{"Hidden":"unused"}}]}),
        );
        assert_eq!(
            value["config"][0],
            json!({"subnet":"10.0.0.0/24", "ipRange":"10.0.0.0/25", "gateway":"10.0.0.1"})
        );
        assert_eq!(value["options"]["Mixed.Option"], "yes");
        assert_eq!(ipam(value.clone()), value);
        let c = network_container(
            json!({"Name":"web", "EndpointID":"ep", "MacAddress":"mac", "IPv4Address":"10.0.0.2/24", "IPv6Address":"::2/64"}),
        );
        assert_eq!(c["ipV4Address"], "10.0.0.2/24");
        assert_eq!(c["endpointId"], "ep");
        assert_eq!(
            peer(json!({"Name":"node", "IP":"10.0.0.3"})),
            json!({"name":"node", "ip":"10.0.0.3"})
        );
        assert!(system_network("HOST", false));
        assert!(system_network("custom", true));
        assert!(!system_network("custom", false));
    }
    #[test]
    fn cluster_details_preserve_int64_values_and_user_keys() {
        let v = cluster_volume(
            json!({"ID":"cluster", "Version":{"Index":9007199254740993_i64}, "Spec":{"Group":"data", "AccessMode":{"Scope":"multi", "Sharing":"readonly", "CapacityRange":{"RequiredBytes":4096}, "Secrets":[{"Key":"Token", "Secret":"secret-id"}]}}, "Info":{"VolumeID":"csi", "VolumeContext":{"Mixed.Key":"value"}, "AccessibleTopology":[{"Zone.Name":"z1"}]}, "PublishStatus":[{"NodeID":"node", "State":"published", "PublishContext":{"Mount.Path":"/data"}}]}),
        );
        assert_eq!(v["version"]["index"], 9007199254740993_i64);
        assert_eq!(v["spec"]["accessMode"]["scope"], "Multi");
        assert_eq!(v["spec"]["accessMode"]["sharing"], "ReadOnly");
        assert_eq!(
            v["info"]["accessibleTopology"][0]["labels"]["Zone.Name"],
            "z1"
        );
        assert_eq!(
            v["publishStatus"][0]["publishContext"]["Mount.Path"],
            "/data"
        );
        assert_eq!(cluster_volume(v.clone()), v);
        assert_eq!(cluster_volume(Value::Null), Value::Null);
    }
}
