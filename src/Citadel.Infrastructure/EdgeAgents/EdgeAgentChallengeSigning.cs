using System.Buffers.Binary;

namespace Infrastructure.EdgeAgents;

internal static class EdgeAgentChallengeSigning
{
    public static byte[] BuildPayload(long timestampUnixSeconds, ReadOnlySpan<byte> nonce)
    {
        var payload = new byte[sizeof(long) + nonce.Length];
        BinaryPrimitives.WriteInt64LittleEndian(payload.AsSpan(0, sizeof(long)), timestampUnixSeconds);
        nonce.CopyTo(payload.AsSpan(sizeof(long)));
        return payload;
    }
}
