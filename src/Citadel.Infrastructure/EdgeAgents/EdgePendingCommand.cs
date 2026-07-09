using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using System.Threading.Channels;

namespace Infrastructure.EdgeAgents;

internal sealed class EdgePendingCommand(string commandId, bool expectsStream)
{
    private readonly Channel<EdgeAgentStreamItem> channel =
        Channel.CreateBounded<EdgeAgentStreamItem>(Helpers.ChannelDefaultOptions(
            capacity: EdgeAgentDefaults.PendingOutputQueueCapacity,
            singleWriter: false));

    public ChannelReader<EdgeAgentStreamItem> Reader => channel.Reader;
    public string CommandId { get; } = commandId;
    public bool ExpectsStream { get; } = expectsStream;

    public bool Output(byte[] payload)
    {
        if (payload.Length > EdgeAgentDefaults.MaxEnvelopePayloadBytes)
        {
            Fail("Edge Agent command output exceeded the maximum payload size.");
            return false;
        }

        if (channel.Writer.TryWrite(EdgeAgentStreamItem.Output(payload)))
        {
            return true;
        }

        Fail("Edge Agent command output queue is full.");
        return false;
    }

    public void Complete()
    {
        channel.Writer.TryWrite(EdgeAgentStreamItem.Complete());
        channel.Writer.TryComplete();
    }

    public void Fail(string message)
    {
        channel.Writer.TryWrite(EdgeAgentStreamItem.Failure(message));
        channel.Writer.TryComplete();
    }
}
