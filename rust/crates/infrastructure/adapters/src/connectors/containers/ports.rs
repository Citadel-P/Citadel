use serde_json::{Map, Value, json};

/// Citadel exposes a port-to-bindings map, not Docker's container-list port array.
/// Also used on reads so projections written before this mapping was fixed work
/// immediately, without a database reset or waiting for reconciliation.
pub(crate) fn normalize(ports: Value) -> Value {
    match ports {
        Value::Array(ports) => {
            let mut mapped = Map::new();
            for port in ports {
                let Some(private_port) = port["PrivatePort"]
                    .as_u64()
                    .filter(|p| (1..=65535).contains(p))
                else {
                    continue;
                };
                let protocol = port["Type"].as_str().unwrap_or("tcp");
                let bindings = mapped
                    .entry(format!("{private_port}/{protocol}"))
                    .or_insert_with(|| json!([]));
                if let Some(public_port) = port["PublicPort"]
                    .as_u64()
                    .filter(|p| (1..=65535).contains(p))
                {
                    bindings
                        .as_array_mut()
                        .expect("bindings are initialized as an array")
                        .push(json!({
                            "hostIP": port["IP"].as_str().unwrap_or_default(),
                            "hostPort": public_port.to_string(),
                        }));
                }
            }
            Value::Object(mapped)
        }
        Value::Object(mut ports) => {
            for bindings in ports.values_mut() {
                if bindings.is_null() {
                    *bindings = json!([]);
                }
                if let Some(bindings) = bindings.as_array_mut() {
                    for binding in bindings {
                        if let Some(binding) = binding.as_object_mut() {
                            for alias in ["hostIp", "HostIp"] {
                                if let Some(ip) = binding.remove(alias) {
                                    binding.entry("hostIP").or_insert(ip);
                                }
                            }
                            if let Some(port) = binding.remove("HostPort") {
                                binding.entry("hostPort").or_insert(port);
                            }
                        }
                    }
                }
            }
            Value::Object(ports)
        }
        _ => json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_ports_preserve_protocols_and_ipv4_ipv6_bindings() {
        assert_eq!(
            normalize(json!([
                {"PrivatePort": 80, "PublicPort": 8080, "Type": "tcp", "IP": "0.0.0.0"},
                {"PrivatePort": 80, "PublicPort": 8080, "Type": "tcp", "IP": "::"},
                {"PrivatePort": 53, "PublicPort": 5353, "Type": "udp", "IP": "127.0.0.1"},
                {"PrivatePort": 443, "Type": "tcp"}
            ])),
            json!({
                "80/tcp": [{"hostIP": "0.0.0.0", "hostPort": "8080"}, {"hostIP": "::", "hostPort": "8080"}],
                "53/udp": [{"hostIP": "127.0.0.1", "hostPort": "5353"}],
                "443/tcp": []
            })
        );
    }

    #[test]
    fn empty_and_exposed_only_ports_are_not_published() {
        for ports in [Value::Null, json!([])] {
            assert_eq!(normalize(ports), json!({}));
        }
        assert_eq!(
            normalize(json!([{"PrivatePort": 80, "IP": "", "Type": "tcp"}])),
            json!({"80/tcp": []})
        );
    }

    #[test]
    fn stored_agent_ports_use_the_public_contract_spelling_and_are_idempotent() {
        let expected = json!({"80/tcp": [{"hostIP": "::", "hostPort": "8080"}], "443/tcp": []});
        assert_eq!(
            normalize(json!({"80/tcp": [{"hostIp": "::", "hostPort": "8080"}], "443/tcp": null})),
            expected
        );
        assert_eq!(normalize(expected.clone()), expected);
        assert_eq!(
            normalize(json!({"80/tcp": [{"HostIp": "::", "HostPort": "8080"}], "443/tcp": null})),
            expected
        );
    }
}
