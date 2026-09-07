use super::*;
use crate::container_terminal::{decode_output, encode_input};
use citadel_platforms::terminal::*;

impl ContainerTerminalPort for AgentClient {
    fn container_terminal<'a>(
        &'a self,
        id: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>> {
        Box::pin(async move {
            let open = crate::container_terminal::opening(id, shell);
            // The exact first protobuf message is authenticated, not an empty streaming body.
            let signed = self.signer.sign(
                open.clone(),
                "/citadel.containers.v1.ContainerService/Exec",
                None,
            )?;
            let (metadata, extensions, _) = signed.into_parts();
            let (input, mut receiver) = input_channel();
            let cancel = cancel.child_token();
            let guard = cancel.clone().drop_guard();
            let input_cancel = cancel.clone();
            let body = async_stream::stream! {
                yield open;
                loop {
                    let item = tokio::select! {biased; ()=input_cancel.cancelled()=>break, item=receiver.recv()=>item};
                    let Some(item)=item else {break};
                    yield encode_input(item);
                }
            };
            let mut client = self.container_client();
            let request = Request::from_parts(metadata, extensions, body);
            let response = tokio::select! {
                biased;
                ()=cancel.cancelled()=>return Err(cancelled_error()),
                result=tokio::time::timeout(self.operation_timeout,client.exec(request))=>
                    result.map_err(|_|timeout_error("opening container terminal"))?.map_err(normalize_status)?,
            };
            let mut output = response.into_inner();
            let output = Box::pin(async_stream::try_stream! {
                let _guard = guard;
                loop {
                    let item=tokio::select! {biased; ()=cancel.cancelled()=>break, item=output.next()=>item};
                    let Some(item)=item else {break};
                    let item=decode_output(item.map_err(normalize_status)?)?;
                    let exited=matches!(item,TerminalOutput::Exit(_));
                    yield item;
                    if exited {break;}
                }
            });
            Ok(TerminalSession { input, output })
        })
    }
}
