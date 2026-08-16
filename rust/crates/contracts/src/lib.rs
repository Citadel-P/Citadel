#![forbid(unsafe_code)]

pub mod citadel {
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
}
