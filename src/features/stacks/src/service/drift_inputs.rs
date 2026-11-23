//! Reuse successful validated membership across pages within one periodic sweep.
use super::*;
use crate::StackDriftContext;
use sha2::{Digest, Sha256};

pub(super) fn compile(files: &[String]) -> Result<BTreeSet<String>, StackError> {
    Ok(parse_compose(files)?
        .services
        .into_iter()
        .map(|s| s.name)
        .collect())
}

#[derive(Default)]
pub struct DriftPass {
    entries: BTreeMap<[u8; 32], Arc<BTreeSet<String>>>,
    bytes: usize,
}
fn field(hash: &mut Sha256, data: &[u8]) {
    hash.update((data.len() as u64).to_be_bytes());
    hash.update(data);
}
impl DriftPass {
    pub(super) async fn load(
        &mut self,
        materializer: Option<&dyn crate::StackSourceMaterializerPort>,
        cancel: &CancellationToken,
        stack: &StackDriftContext,
        actor: ActorId,
    ) -> Result<Arc<BTreeSet<String>>, StackError> {
        let mut hash = Sha256::new();
        let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
        match spec {
            StackSpec::WebEditor { compose_file, .. } => {
                hash.update([0]);
                field(&mut hash, compose_file.as_bytes());
            }
            StackSpec::Git {
                git_repo_id,
                compose_paths,
                ..
            } => {
                hash.update([1]);
                field(&mut hash, git_repo_id.as_bytes());
                field(
                    &mut hash,
                    stack
                        .deployed_commit
                        .as_deref()
                        .unwrap_or_default()
                        .as_bytes(),
                );
                for path in compose_paths {
                    field(&mut hash, path.as_bytes());
                }
            }
        }
        let key = hash.finalize().into();
        if let Some(names) = self.entries.get(&key) {
            return Ok(names.clone());
        }
        let names = Arc::new(match spec {
            StackSpec::WebEditor { compose_file, .. } => {
                compile(std::slice::from_ref(compose_file))?
            }
            StackSpec::Git { .. } => {
                let claim = drift_source_claim(stack, actor)?;
                let source = materializer.ok_or_else(|| {
                    StackError::Runtime("Git Stack source reading is unavailable.".into())
                })?;
                compile(&source.compose_contents(&claim, cancel).await?)?
            }
        });
        // Include allowance for allocation and tree nodes, including empty models.
        let size = 128 + names.iter().map(|s| s.capacity() + 96).sum::<usize>();
        if self.entries.len() < 100 && size <= 1024 * 1024 - self.bytes {
            self.entries.insert(key, names.clone());
            self.bytes += size;
        }
        Ok(names)
    }
}
