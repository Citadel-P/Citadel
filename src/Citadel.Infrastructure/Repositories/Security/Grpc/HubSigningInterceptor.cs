using System.Buffers;
using System.Buffers.Binary;
using Google.Protobuf;
using Grpc.Core;
using Grpc.Core.Interceptors;
using Hosting.Common;
using NSec.Cryptography;

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
public class HubSigningInterceptor : Interceptor
{
    private readonly SignatureAlgorithm algo;
    private readonly Key hubPrivateKey;
    private const int NonceSize = 8;

    public HubSigningInterceptor()
    {
        hubPrivateKey = Helpers.GetOrCreatePrivateKey();
        algo = SignatureAlgorithm.Ed25519;
    }
     
    public override AsyncUnaryCall<TResponse> AsyncUnaryCall<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context,
        AsyncUnaryCallContinuation<TRequest, TResponse> continuation)
    {
        var newContext = PrepareSignedContext(request, context);
        return continuation(request, newContext);
    }

    public override AsyncServerStreamingCall<TResponse> AsyncServerStreamingCall<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context,
        AsyncServerStreamingCallContinuation<TRequest, TResponse> continuation)
    {
        var newContext = PrepareSignedContext(request, context);
        return continuation(request, newContext);
    }

    private ClientInterceptorContext<TRequest, TResponse> PrepareSignedContext<TRequest, TResponse>(
        TRequest request,
        ClientInterceptorContext<TRequest, TResponse> context)
        where TRequest : class
        where TResponse : class
    {
        if (request is not IMessage protoMessage)
            throw new InvalidOperationException("TRequest must implement IMessage");

        int messageSize = protoMessage.CalculateSize();
        int totalSize = NonceSize + messageSize;
        byte[] buffer = ArrayPool<byte>.Shared.Rent(totalSize);

        try
        {
            var span = buffer.AsSpan(0, totalSize);
            var nonceSpan = span[..NonceSize];
            var messageSpan = span.Slice(NonceSize, messageSize);

            // Fill nonce with current Unix timestamp
            BinaryPrimitives.WriteInt64LittleEndian(nonceSpan, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

            // Serialize protobuf directly into the buffer
            protoMessage.WriteTo(messageSpan);

            // Sign nonce + message
            byte[] signature = algo.Sign(hubPrivateKey, span);

            // Ensure headers exist and add signature + nonce
            var headers = context.Options.Headers ?? [];
            headers.Add(Constants.NonceHeaderKey, nonceSpan.ToArray());
            headers.Add(Constants.SignatureHeaderKey, signature);

            var newOptions = context.Options.WithHeaders(headers);
            return new ClientInterceptorContext<TRequest, TResponse>(context.Method, context.Host, newOptions);
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(buffer);
        }
    }
}