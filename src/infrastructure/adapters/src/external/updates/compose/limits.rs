//! Resource ceilings retained by Compose replacements and checked against Docker.
use super::resolution::unsupported;
use citadel_updates::UpdateError;
use serde_json::Value;

pub(in crate::external::updates) fn verify_limits(
    service: &Value,
    host: &Value,
) -> Result<(), UpdateError> {
    let invalid =
        || unsupported("Running CPU or memory limits differ from the registered Compose inputs.");
    let cpus = match service.get("cpus") {
        None => 0.0,
        Some(value) => value
            .as_f64()
            .or_else(|| value.as_str()?.parse().ok())
            .ok_or_else(invalid)?,
    };
    if !cpus.is_finite() || cpus < 0.0 || cpus * 1e9 >= i64::MAX as f64 {
        return Err(invalid());
    }
    let memory = match service.get("mem_limit") {
        None => 0,
        Some(value) => compose_integer(value)
            .filter(|v| *v >= 0)
            .ok_or_else(invalid)?,
    };
    let swap = match service.get("memswap_limit") {
        None => 0,
        Some(value) => compose_integer(value)
            .filter(|v| *v >= -1)
            .ok_or_else(invalid)?,
    };
    let swap = if swap == 0 && memory > 0 {
        memory.checked_mul(2).ok_or_else(invalid)?
    } else {
        swap
    };
    for (field, expected) in [
        ("NanoCpus", (cpus * 1e9) as i64),
        ("Memory", memory),
        ("MemorySwap", swap),
    ] {
        let actual = match host.get(field) {
            None => 0,
            Some(value) => value.as_i64().ok_or_else(invalid)?,
        };
        if actual != expected {
            return Err(invalid());
        }
    }
    Ok(())
}

// Compose serializes normalized byte quantities as decimal strings; Docker
// inspection reports numeric integers. Accept either Compose representation.
fn compose_integer(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checks_cpu_and_memory_limits_including_docker_swap_default() {
        for (service, host) in [
            (
                json!({"cpus": 0.75, "mem_limit": "268435456", "memswap_limit": "268435456"}),
                json!({"NanoCpus":750000000,"Memory":268435456,"MemorySwap":268435456}),
            ),
            (
                json!({"cpus": 2.0, "mem_limit": 0, "memswap_limit": 0}),
                json!({"NanoCpus":2000000000,"Memory":0,"MemorySwap":0}),
            ),
            (
                json!({"mem_limit": 1024}),
                json!({"Memory":1024,"MemorySwap":2048}),
            ),
            (
                json!({"mem_limit": 1024, "memswap_limit": -1}),
                json!({"Memory":1024,"MemorySwap":-1}),
            ),
        ] {
            verify_limits(&service, &host).unwrap();
            for key in ["NanoCpus", "Memory", "MemorySwap"] {
                let mut changed = host.clone();
                changed[key] = json!(host[key].as_i64().unwrap_or(0) + 1);
                assert!(verify_limits(&service, &changed).is_err());
            }
        }
    }

    #[test]
    fn rejects_invalid_values_and_unrecorded_live_limits() {
        for value in [
            json!({"cpus": -1}),
            json!({"cpus":"oops"}),
            json!({"mem_limit":-1}),
            json!({"memswap_limit":-2}),
        ] {
            assert!(verify_limits(&value, &json!({})).is_err());
        }
        assert!(verify_limits(&json!({}), &json!({"Memory":1024})).is_err());
    }
}
