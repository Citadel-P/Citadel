use crate::{
    DashboardContent, DashboardError, DashboardSharingMode, DashboardSummary, DashboardViewer,
    DashboardVisibility, SavedDashboard,
};
use futures_util::future::BoxFuture;
use uuid::Uuid;

type ResultFuture<'a, T> = BoxFuture<'a, Result<T, DashboardError>>;

pub trait DashboardRepository: Send + Sync {
    fn fleet(
        &self,
        viewer: DashboardViewer,
        filter: crate::FleetFilter,
    ) -> ResultFuture<'_, crate::FleetPage>;
    fn sources(
        &self,
        viewer: DashboardViewer,
        filter: crate::SourceFilter,
    ) -> ResultFuture<'_, Vec<crate::DashboardSource>>;
    fn list<'a>(
        &'a self,
        viewer: DashboardViewer,
        search: &'a str,
        limit: i64,
        offset: i64,
    ) -> ResultFuture<'a, Vec<DashboardSummary>>;
    fn get(&self, viewer: DashboardViewer, id: Uuid) -> ResultFuture<'_, SavedDashboard>;
    fn create(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        visibility: DashboardVisibility,
        content: DashboardContent,
    ) -> ResultFuture<'_, SavedDashboard>;
    fn update(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        revision: i64,
        content: DashboardContent,
    ) -> ResultFuture<'_, SavedDashboard>;
    fn delete(&self, viewer: DashboardViewer, id: Uuid, revision: i64) -> ResultFuture<'_, ()>;
    fn set_visibility(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        revision: i64,
        mode: DashboardSharingMode,
        custom_access: bool,
    ) -> ResultFuture<'_, DashboardSummary>;
    fn default_for(&self, viewer: DashboardViewer) -> ResultFuture<'_, Option<Uuid>>;
    fn set_default(
        &self,
        viewer: DashboardViewer,
        instance: bool,
        id: Option<Uuid>,
    ) -> ResultFuture<'_, ()>;
}
