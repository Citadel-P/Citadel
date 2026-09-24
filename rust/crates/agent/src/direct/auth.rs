//! Authenticate the first protobuf frame before decoding can reorder map entries.
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use bytes::BytesMut;
use ed25519_dalek::{Signature, VerifyingKey};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tonic::{Status, metadata::MetadataMap};

const CLOCK_SKEW: i64 = 60;
const NONCE_CAPACITY: usize = 65_536;

pub struct Verifier {
    key: VerifyingKey,
    shutdown: tokio_util::sync::CancellationToken,
    nonces: Mutex<HashMap<[u8; 16], i64>>,
}

impl Verifier {
    pub fn new(key: VerifyingKey) -> Self {
        Self {
            key,
            shutdown: tokio_util::sync::CancellationToken::new(),
            nonces: Mutex::new(HashMap::new()),
        }
    }

    pub fn with_shutdown(mut self, shutdown: tokio_util::sync::CancellationToken) -> Self {
        self.shutdown = shutdown;
        self
    }

    fn verify(
        &self,
        metadata: &MetadataMap,
        method: &str,
        body: &[u8],
        now: i64,
    ) -> Result<(), Status> {
        let timestamp = binary::<8>(metadata, "x-timestamp-bin")?;
        let nonce = binary::<16>(metadata, "x-nonce-bin")?;
        let hash = binary::<32>(metadata, "x-content-sha256-bin")?;
        let signature = binary::<64>(metadata, "x-signature-bin")?;
        let time = i64::from_le_bytes(timestamp);
        if time.abs_diff(now) > CLOCK_SKEW as u64 {
            return Err(Status::unauthenticated("Request expired"));
        }
        if !bool::from(Sha256::digest(body).as_slice().ct_eq(&hash)) {
            return Err(Status::unauthenticated("Payload hash mismatch"));
        }
        let mut signed = Vec::with_capacity(56 + method.len());
        signed.extend_from_slice(&timestamp);
        signed.extend_from_slice(&nonce);
        signed.extend_from_slice(method.as_bytes());
        signed.extend_from_slice(&hash);
        self.key
            .verify_strict(&signed, &Signature::from_bytes(&signature))
            .map_err(|_| Status::unauthenticated("Invalid signature"))?;
        let mut nonces = self
            .nonces
            .lock()
            .map_err(|_| Status::internal("Replay protection unavailable"))?;
        nonces.retain(|_, expires| *expires >= now);
        if nonces.contains_key(&nonce) {
            return Err(Status::unauthenticated("Replay attack detected"));
        }
        // Fail closed instead of evicting a nonce whose signature remains valid.
        if nonces.len() >= NONCE_CAPACITY {
            return Err(Status::resource_exhausted(
                "Replay protection capacity reached",
            ));
        }
        nonces.insert(nonce, time.saturating_add(CLOCK_SKEW));
        Ok(())
    }
}

fn binary<const N: usize>(metadata: &MetadataMap, key: &'static str) -> Result<[u8; N], Status> {
    let mut values = metadata.get_all_bin(key).iter();
    let value = values
        .next()
        .ok_or_else(|| Status::unauthenticated("Missing security headers"))?;
    if values.next().is_some() {
        return Err(Status::unauthenticated("Duplicate security headers"));
    }
    value
        .to_bytes()
        .ok()
        .and_then(|bytes| bytes.as_ref().try_into().ok())
        .ok_or_else(|| Status::unauthenticated("Invalid security headers"))
}

pub async fn authenticate(
    State(verifier): State<std::sync::Arc<Verifier>>,
    request: Request,
    next: Next,
) -> Response {
    let deadline = match request
        .headers()
        .get("grpc-timeout")
        .map(parse_timeout)
        .transpose()
    {
        Ok(value) => value.and_then(|duration| tokio::time::Instant::now().checked_add(duration)),
        Err(status) => return status.into_http(),
    };
    let shutdown = verifier.shutdown.clone();
    let call = async {
        match authenticated(verifier, request).await {
            Ok(request) => next.run(request).await,
            Err(status) => status.into_http(),
        }
    };
    let call = async {
        tokio::select! { ()=shutdown.cancelled()=>Status::cancelled("Agent shutting down").into_http(), response=call=>response }
    };
    let Some(deadline) = deadline else {
        return call.await;
    };
    let response = match tokio::time::timeout_at(deadline, call).await {
        Ok(response) => response,
        Err(_) => return Status::deadline_exceeded("Request deadline exceeded").into_http(),
    };
    let (parts, mut body) = response.into_parts();
    let stream = async_stream::stream! {
        loop {
            match tokio::time::timeout_at(deadline,body.frame()).await {
                Ok(Some(frame))=>yield frame,
                Ok(None)=>break,
                Err(_)=>{
                    let mut trailers=axum::http::HeaderMap::new();
                    Status::deadline_exceeded("Request deadline exceeded").add_header(&mut trailers).expect("valid status");
                    yield Ok(http_body::Frame::trailers(trailers));
                    break;
                }
            }
        }
    };
    Response::from_parts(parts, Body::new(http_body_util::StreamBody::new(stream)))
}
fn parse_timeout(value: &axum::http::HeaderValue) -> Result<std::time::Duration, Status> {
    let invalid = || Status::invalid_argument("Invalid gRPC timeout");
    let value = value.to_str().map_err(|_| invalid())?;
    if !(2..=9).contains(&value.len()) {
        return Err(invalid());
    }
    let (amount, unit) = value.split_at(value.len() - 1);
    if !amount.bytes().all(|v| v.is_ascii_digit()) {
        return Err(invalid());
    }
    let amount = amount.parse::<u64>().map_err(|_| invalid())?;
    Ok(match unit {
        "H" => std::time::Duration::from_secs(amount * 3600),
        "M" => std::time::Duration::from_secs(amount * 60),
        "S" => std::time::Duration::from_secs(amount),
        "m" => std::time::Duration::from_millis(amount),
        "u" => std::time::Duration::from_micros(amount),
        "n" => std::time::Duration::from_nanos(amount),
        _ => return Err(invalid()),
    })
}

async fn authenticated(
    verifier: std::sync::Arc<Verifier>,
    request: Request,
) -> Result<Request, Status> {
    let (parts, body) = request.into_parts();
    let metadata = MetadataMap::from_headers(parts.headers.clone());
    // Reject malformed metadata before waiting for a potentially idle body.
    binary::<8>(&metadata, "x-timestamp-bin")?;
    binary::<16>(&metadata, "x-nonce-bin")?;
    binary::<32>(&metadata, "x-content-sha256-bin")?;
    binary::<64>(&metadata, "x-signature-bin")?;
    let mut stream = body.into_data_stream();
    let mut opening = BytesMut::new();
    let size = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if opening.len() >= 5 {
                if opening[0] != 0 {
                    return Err(Status::unimplemented(
                        "Compressed request messages are not supported",
                    ));
                }
                let size = u32::from_be_bytes(opening[1..5].try_into().expect("four length bytes"))
                    as usize;
                if size > super::MAX_MESSAGE_BYTES {
                    return Err(Status::resource_exhausted("Request exceeds 16 MiB"));
                }
                if opening.len() >= size + 5 {
                    return Ok(size);
                }
            }
            let chunk = stream
                .next()
                .await
                .ok_or_else(|| Status::unauthenticated("Missing signed opening message"))?
                .map_err(|_| Status::cancelled("Request body disconnected"))?;
            opening.extend_from_slice(&chunk);
        }
    })
    .await
    .map_err(|_| Status::deadline_exceeded("Opening message timed out"))??;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Status::internal("System clock unavailable"))?
        .as_secs() as i64;
    verifier.verify(&metadata, parts.uri.path(), &opening[5..5 + size], now)?;
    let restored =
        futures_util::stream::once(async move { Ok::<_, axum::Error>(opening.freeze()) })
            .chain(stream);
    Ok(Request::from_parts(parts, Body::from_stream(restored)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use tonic::metadata::MetadataValue;

    fn signed(body: &[u8], method: &str, timestamp: i64, nonce: [u8; 16]) -> MetadataMap {
        let key = SigningKey::from_bytes(&[42; 32]);
        let timestamp = timestamp.to_le_bytes();
        let hash = Sha256::digest(body);
        let payload = [&timestamp[..], &nonce, method.as_bytes(), &hash].concat();
        let mut metadata = MetadataMap::new();
        for (name, bytes) in [
            ("x-timestamp-bin", &timestamp[..]),
            ("x-nonce-bin", &nonce),
            ("x-content-sha256-bin", &hash),
            ("x-signature-bin", &key.sign(&payload).to_bytes()),
        ] {
            metadata.insert_bin(name, MetadataValue::from_bytes(bytes));
        }
        metadata
    }
    fn verifier() -> Verifier {
        Verifier::new(SigningKey::from_bytes(&[42; 32]).verifying_key())
    }

    #[test]
    fn method_body_timestamp_and_key_are_authenticated_before_nonce_is_consumed() {
        let method = "/citadel.volumes.v1.VolumeService/Create";
        let v = verifier();
        let metadata = signed(b"payload", method, 1000, [1; 16]);
        for (path, body, now) in [
            ("/other", &b"payload"[..], 1000),
            (method, &b"changed"[..], 1000),
            (method, &b"payload"[..], 1061),
        ] {
            assert_eq!(
                v.verify(&metadata, path, body, now).unwrap_err().code(),
                tonic::Code::Unauthenticated
            );
        }
        assert!(v.verify(&metadata, method, b"payload", 1060).is_ok());
        assert_eq!(
            v.verify(&metadata, method, b"payload", 1060)
                .unwrap_err()
                .code(),
            tonic::Code::Unauthenticated
        );
        let other = Verifier::new(SigningKey::from_bytes(&[43; 32]).verifying_key());
        assert!(other.verify(&metadata, method, b"payload", 1000).is_err());
    }

    #[test]
    fn future_timestamps_keep_nonces_until_the_last_acceptable_second() {
        let v = verifier();
        let metadata = signed(b"", "/call", 1060, [2; 16]);
        v.verify(&metadata, "/call", b"", 1000).unwrap();
        assert!(v.verify(&metadata, "/call", b"", 1120).is_err());
        // A newly signed use of the same nonce remains rejected until expiry.
        let next = signed(b"", "/call", 1121, [2; 16]);
        assert!(v.verify(&next, "/call", b"", 1121).is_ok());
        for timestamp in [i64::MIN, i64::MAX, 939, 1061] {
            assert!(
                v.verify(
                    &signed(b"", "/call", timestamp, [3; 16]),
                    "/call",
                    b"",
                    1000
                )
                .is_err()
            );
        }
    }

    #[test]
    fn malformed_or_duplicate_headers_are_rejected() {
        for (name, length) in [
            ("x-timestamp-bin", 8),
            ("x-nonce-bin", 16),
            ("x-content-sha256-bin", 32),
            ("x-signature-bin", 64),
        ] {
            for bad in [length - 1, length + 1] {
                let mut metadata = signed(b"", "/call", 1000, [1; 16]);
                metadata.insert_bin(name, MetadataValue::from_bytes(&vec![0; bad]));
                assert!(verifier().verify(&metadata, "/call", b"", 1000).is_err());
            }
            let mut metadata = signed(b"", "/call", 1000, [1; 16]);
            metadata.append_bin(name, MetadataValue::from_bytes(&vec![0; length]));
            assert!(verifier().verify(&metadata, "/call", b"", 1000).is_err());
        }
        assert!(
            verifier()
                .verify(&MetadataMap::new(), "/call", b"", 1000)
                .is_err()
        );
    }

    #[test]
    fn concurrent_replays_have_exactly_one_winner_and_capacity_fails_closed() {
        let v = std::sync::Arc::new(verifier());
        let metadata = signed(b"", "/call", 1000, [1; 16]);
        let winners = std::thread::scope(|scope| {
            let tasks: Vec<_> = (0..16)
                .map(|_| scope.spawn(|| v.verify(&metadata, "/call", b"", 1000).is_ok()))
                .collect();
            tasks
                .into_iter()
                .map(|task| usize::from(task.join().unwrap()))
                .sum::<usize>()
        });
        assert_eq!(winners, 1);
        v.nonces.lock().unwrap().clear();
        v.nonces
            .lock()
            .unwrap()
            .extend((0..NONCE_CAPACITY).map(|i| ((i as u128).to_le_bytes(), 1060)));
        let metadata = signed(b"", "/call", 1000, [255; 16]);
        assert_eq!(
            v.verify(&metadata, "/call", b"", 1000).unwrap_err().code(),
            tonic::Code::ResourceExhausted
        );
        assert_eq!(v.nonces.lock().unwrap().len(), NONCE_CAPACITY);
    }
}
