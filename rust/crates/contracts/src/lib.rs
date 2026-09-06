#![forbid(unsafe_code)]

pub mod http;

pub mod citadel {
    pub mod edge {
        pub mod v1 {
            tonic::include_proto!("citadel.edge.v1");
        }
    }
    pub mod shared_models {
        pub mod v1 {
            tonic::include_proto!("citadel.shared_models.v1");
        }
    }

    pub mod platforms {
        pub mod v1 {
            #![allow(clippy::large_enum_variant)]
            tonic::include_proto!("citadel.platforms.v1");
        }
    }

    pub mod containers {
        pub mod v1 {
            tonic::include_proto!("citadel.containers.v1");
        }
    }

    pub mod deployments {
        pub mod v1 {
            tonic::include_proto!("citadel.deployments.v1");
        }
    }

    pub mod stacks {
        pub mod v1 {
            tonic::include_proto!("citadel.stacks.v1");
        }
    }

    pub mod images {
        pub mod v1 {
            tonic::include_proto!("citadel.images.v1");
        }
    }

    pub mod networks {
        pub mod v1 {
            tonic::include_proto!("citadel.networks.v1");
        }
    }

    pub mod volumes {
        pub mod v1 {
            tonic::include_proto!("citadel.volumes.v1");
        }
    }

    pub mod swarm {
        pub mod v1 {
            tonic::include_proto!("citadel.swarm.v1");
        }
    }
}
