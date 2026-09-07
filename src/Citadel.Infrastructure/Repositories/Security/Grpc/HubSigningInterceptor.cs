using Google.Protobuf;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Hosting.Common;
using NSec.Cryptography;
using System.Buffers.Binary;
using System.Security.Cryptography;

namespace Infrastructure.Repositories.Security.Grpc;

/// <summary>
/// - Signs every outgoing gRPC request by computing an Ed25519 signature over the concatenation of a nonce (timestamp) and the serialized request bytes. 
///     The nonce and signature are sent in gRPC metadata headers (x-nonce-bin and x-signature-bin) to ensure authenticity, integrity, and replay protection.
/// - Attaches nonce(timestamp) and signature in gRPC metadata headers(x-nonce-bin and x-signature-bin).
/// - Supports unary and server-streaming RPCs.
/// </summary>
/// <remarks>
/// - Single private key per Citadel instance: rotating keys requires updating the agent(s).
/// - Relies on deterministic serialization: protobuf maps may still produce slightly different byte orders if versions differ, so both client and server must use the same .NET/Protobuf version.
/// - Does not sign streamed responses: only signs the request, not the server’s stream.
/// </remarks>
public sealed class HubSigningInterceptor : Interceptor
{
    public override AsyncUnaryCall<TResponse> AsyncUnaryCall<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context,
        AsyncUnaryCallContinuation<TRequest, TResponse> continuation)
    {
        var signedContext = SignRequest(request, context);

        return continuation(request, signedContext);
    }

    public override AsyncServerStreamingCall<TResponse> AsyncServerStreamingCall<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context,
        AsyncServerStreamingCallContinuation<TRequest, TResponse> continuation)
    {
        var signedContext = SignRequest(request, context);

        return continuation(request, signedContext);
    }

    private ClientInterceptorContext<TRequest, TResponse> SignRequest<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context)
        where TRequest : class
        where TResponse : class
    {
        if (request is not IMessage proto)
            throw new InvalidOperationException(
                "TRequest must implement IMessage");

        var headers = SignHeaders(proto, context.Method.FullName, context.Options.Headers);
        return new ClientInterceptorContext<TRequest, TResponse>(
            context.Method, context.Host, context.Options.WithHeaders(headers));
    }

    // Duplex calls authenticate their first message before opening the stream.
    internal static Metadata SignHeaders(IMessage proto, string method, Metadata? headers = null)
    {
        long timestamp =
            DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        byte[] nonce =
            RandomNumberGenerator.GetBytes(Constants.GrpcRequestMetadata.NonceSize);

        byte[] bodyHash =
            Helpers.RequestSigning.ComputeSha256(proto);

        byte[] signedPayload =
            Helpers.RequestSigning.BuildSignedPayload(
                timestamp,
                nonce,
                method,
                bodyHash);

        using var privateKey = Helpers.GetOrCreatePrivateKey();
        byte[] signature =
            SignatureAlgorithm.Ed25519.Sign(privateKey, signedPayload);

        headers ??= new Metadata();

        Span<byte> timestampBytes = stackalloc byte[Constants.GrpcRequestMetadata.TimestampSize];

        BinaryPrimitives.WriteInt64LittleEndian(
            timestampBytes,
            timestamp);

        headers.Add(Constants.GrpcRequestMetadata.TimestampHeaderKey, timestampBytes.ToArray());
        headers.Add(Constants.GrpcRequestMetadata.NonceHeaderKey, nonce);
        headers.Add(Constants.GrpcRequestMetadata.ContentHashHeaderKey, bodyHash);
        headers.Add(Constants.GrpcRequestMetadata.SignatureHeaderKey, signature);

        CryptographicOperations.ZeroMemory(signedPayload);

        return headers;
    }
}
