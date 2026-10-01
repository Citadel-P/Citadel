//! Finite read capabilities; transport dispatch stays inside the adapter.
use super::runtime::Runtime;
use citadel_platforms::{
    RuntimeCapabilityError,
    containers::ContainerInspectionPort,
    images::{ImageInspection, ImageInspectionPort},
    logs::{LogReadPort, LogResource, LogSnapshot},
};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

impl ContainerInspectionPort for Runtime<'_> {
    fn inspection<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<serde_json::Value, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => ContainerInspectionPort::inspection(*runtime, id, cancel),
            Runtime::Agent(runtime) => ContainerInspectionPort::inspection(runtime, id, cancel),
            Runtime::Edge(runtime) => ContainerInspectionPort::inspection(runtime, id, cancel),
        }
    }
}

impl ImageInspectionPort for Runtime<'_> {
    fn exposed_ports<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => ImageInspectionPort::exposed_ports(*runtime, id, cancel),
            Runtime::Agent(runtime) => ImageInspectionPort::exposed_ports(runtime, id, cancel),
            Runtime::Edge(runtime) => ImageInspectionPort::exposed_ports(runtime, id, cancel),
        }
    }

    fn inspect_image<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ImageInspection, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => ImageInspectionPort::inspect_image(*runtime, id, cancel),
            Runtime::Agent(runtime) => ImageInspectionPort::inspect_image(runtime, id, cancel),
            Runtime::Edge(runtime) => ImageInspectionPort::inspect_image(runtime, id, cancel),
        }
    }
}

impl LogReadPort for Runtime<'_> {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>> {
        match self {
            Runtime::Local(runtime) => LogReadPort::read_logs(*runtime, resource, tail, cancel),
            Runtime::Agent(runtime) => LogReadPort::read_logs(runtime, resource, tail, cancel),
            Runtime::Edge(runtime) => LogReadPort::read_logs(runtime, resource, tail, cancel),
        }
    }
}
