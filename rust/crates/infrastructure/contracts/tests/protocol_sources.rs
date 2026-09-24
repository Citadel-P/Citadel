use sha2::{Digest, Sha256};

#[test]
fn protocol_sources_preserve_the_accepted_wire_contract() {
    let sources = [
        (
            "edge_agent_service.proto",
            include_str!("../proto/edge_agent_service.proto"),
            "f3280c2f341ad5860ed5b34f2e4371356052aa1b66204957e6f0de177efaff32",
        ),
        (
            "shared_models.proto",
            include_str!("../proto/shared_models.proto"),
            "11868d2c8c738f552a3ec8fb0789f4b32909234f6aa8200bb48960d556bf8e5a",
        ),
        (
            "platform_service.proto",
            include_str!("../proto/platform_service.proto"),
            "c8d4c61b5122a39ac5ff3b247c4f3423fa470598d608a97ed8d77310709df66b",
        ),
        (
            "container_service.proto",
            include_str!("../proto/container_service.proto"),
            "79b84dfb6c760b4d60eabcfc20a9cb8111103364d72f9528b22821b08656587d",
        ),
        (
            "deployment_service.proto",
            include_str!("../proto/deployment_service.proto"),
            "df073b4604ff9f8242535ba6661f37e3f811b36a67c9b84da2a4256bb9720cbc",
        ),
        (
            "stack_service.proto",
            include_str!("../proto/stack_service.proto"),
            "4deb6439dccc4d2e659af7be12f81c90969fa165bbc9cf7f35a5b66dca89f9de",
        ),
        (
            "image_service.proto",
            include_str!("../proto/image_service.proto"),
            "89d8485435a0e365d1d02e2a3bbf4a12b5eaf82dfb0e0939eaf61859794e2944",
        ),
        (
            "network_service.proto",
            include_str!("../proto/network_service.proto"),
            "a8ace023dd582a007f74b211f69acfd28bb8ecb86813b44b8026b3df340e21bd",
        ),
        (
            "volume_service.proto",
            include_str!("../proto/volume_service.proto"),
            "270e53f225b16abacc1d63cdb5e23e1efdb1142a6b9cb9efc0d43f7a31d7a2f2",
        ),
        (
            "swarm_service.proto",
            include_str!("../proto/swarm_service.proto"),
            "994ef109395cfaf35d4491ab1fdd8931829fe3665dd4ba38670277285471dfbe",
        ),
    ];
    for (name, source, expected) in sources {
        let normalized = source.replace("\r\n", "\n");
        let actual: String = Sha256::digest(normalized.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            actual, expected,
            "protocol source changed: {name}; review wire compatibility before updating the baseline"
        );
    }
    assert_eq!(citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION, 2);
}
