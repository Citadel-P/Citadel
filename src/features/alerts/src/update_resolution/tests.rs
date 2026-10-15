use super::*;
use serde_json::json;

#[test]
fn image_resolution_requires_every_service_and_exact_image_identity() {
    let state = VerifiedUpdate::StackImages(vec![AppliedStackImage {
        service: "web".into(),
        image: "nginx:alpine".into(),
        digest: "sha256:abc".into(),
    }]);
    let target =
        json!({"ServiceName":"web", "ImageName":"nginx:alpine", "LatestDigest":"sha256:abc"});
    assert!(state.satisfies(&json!({"Updates":[target.clone()]})));
    for other in [
        json!({"ServiceName":"db", "ImageName":"nginx:alpine", "LatestDigest":"sha256:abc"}),
        json!({"ServiceName":"web", "ImageName":"nginx:stable", "LatestDigest":"sha256:abc"}),
        json!({"ServiceName":"web", "ImageName":"nginx:alpine", "LatestDigest":"sha256:newer"}),
    ] {
        assert!(!state.satisfies(&json!({"Updates":[target.clone(), other]})));
    }
    for info in [json!({}), json!({"Updates":[]}), json!({"Updates":[{}]})] {
        assert!(!state.satisfies(&info));
    }
}

#[test]
fn deployment_and_git_resolution_never_accept_unknown_or_different_targets() {
    let image = VerifiedUpdate::DeploymentImage("sha256:abc".into());
    assert!(image.satisfies(&json!({"LatestImage":"sha256:abc"})));
    assert!(!image.satisfies(&json!({"LatestImage":"sha256:newer"})));
    assert!(!image.satisfies(&json!({})));
    assert!(!VerifiedUpdate::DeploymentImage(String::new()).satisfies(&json!({"LatestImage":""})));
    let git = VerifiedUpdate::StackGit("abc123".into());
    assert!(git.satisfies(&json!({"RemoteCommitSha":"abc123"})));
    assert!(!git.satisfies(&json!({"RemoteCommitSha":"def456"})));
    assert!(!VerifiedUpdate::StackGit(String::new()).satisfies(&json!({"RemoteCommitSha":""})));
    assert!(!is_update_available("StackAutoDeployFailed"));
    assert!(!is_update_available("DeploymentAutoUpdated"));
}
