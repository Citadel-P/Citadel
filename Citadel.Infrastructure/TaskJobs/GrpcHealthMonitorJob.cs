using System.Collections.Concurrent;
using System.Threading.Channels;
using Infrastructure.Services.Abstractions;
using Microsoft.Extensions.Hosting;

namespace Infrastructure.TaskJobs;

/// <summary>
/// Monitor gRPC service (Docker Agents) health status.
/// </summary>
public interface IGrpcHealthMonitorJob : IHostedService
{
    bool IsServiceOnline(string address);
    void TrackAddress(string address);
    void TrackAddress(string[] addressess);
    void UntrackAddress(string address);
    event EventHandler<GrpcServiceHealthChangedEventArgs> StatusChanged;
    ChannelReader<GrpcServiceHealthChangedEventArgs> StatusUpdates { get; }
}

internal class GrpcHealthMonitorJob(IGrpcClientFactory clientFactory) : BackgroundService, IGrpcHealthMonitorJob
{
    private readonly Lock @lock = new();
    private readonly List<string> trackedAddresses = [];
    private readonly ConcurrentDictionary<string, bool> status = new();
    private readonly TimeSpan checkInterval = TimeSpan.FromSeconds(5);

    // debounce settings
    private const int FailThreshold = 3;
    private const int SuccessThreshold = 2;

    private readonly Dictionary<string, int> _failureCounts = [];
    private readonly Dictionary<string, int> _successCounts = [];

    private readonly Channel<GrpcServiceHealthChangedEventArgs> _channel =
        Channel.CreateUnbounded<GrpcServiceHealthChangedEventArgs>(new UnboundedChannelOptions
        {
            SingleWriter = true,
            AllowSynchronousContinuations = false
        });

    public event EventHandler<GrpcServiceHealthChangedEventArgs>? StatusChanged;
    public ChannelReader<GrpcServiceHealthChangedEventArgs> StatusUpdates => _channel.Reader;

    public void TrackAddress(string address)
    {
        lock (@lock)
        {
            if (!trackedAddresses.Contains(address))
                trackedAddresses.Add(address);
        }
    }

    public void TrackAddress(string[] addresses)
    {
        lock (@lock)
        {
            foreach (var address in addresses)
            {
                if (!trackedAddresses.Contains(address))
                    trackedAddresses.AddRange(address);
            }
        }
    }

    public void UntrackAddress(string address)
    {
        lock (@lock)
        {
            if (trackedAddresses.Any(s => s == address))
            {
                trackedAddresses.Remove(address);
                status.TryRemove(address, out _);
            }
        }
    }

    public bool IsServiceOnline(string address) =>
        status.TryGetValue(address, out var online) && online;

    protected override async Task ExecuteAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            string[] addresses;
            lock (@lock)
            {
                addresses = [.. trackedAddresses];
            }

            foreach (var address in addresses)
            {
                var isOnline = await ProbeAsync(address);
                var wasOnline = status.TryGetValue(address, out var prevOnline) && prevOnline;

                if (isOnline)
                {
                    _failureCounts[address] = 0;

                    _successCounts[address] = _successCounts.GetValueOrDefault(address) + 1;
                    if (!wasOnline && _successCounts[address] >= SuccessThreshold)
                    {
                        UpdateStatus(address, isOnline, cancellationToken);
                    }
                }
                else
                {
                    _successCounts[address] = 0;

                    _failureCounts[address] = _failureCounts.GetValueOrDefault(address) + 1;
                    if (wasOnline && _failureCounts[address] >= FailThreshold)
                    {
                        UpdateStatus(address, isOnline, cancellationToken);
                    }
                }
            }

            await Task.Delay(checkInterval, cancellationToken);
        }

        _channel.Writer.TryComplete();
    }

    private async void UpdateStatus(string address, bool isOnline, CancellationToken cancellationToken)
    {
        status[address] = isOnline;
        var evt = new GrpcServiceHealthChangedEventArgs(address, isOnline);

        StatusChanged?.Invoke(this, evt);
        await _channel.Writer.WriteAsync(evt, cancellationToken);
    }

    private async Task<bool> ProbeAsync(string address)
    {
        try
        {
            var client = clientFactory.GetPlatformClient(address);
            var response = await client.HealthCheckAsync(new Google.Protobuf.WellKnownTypes.Empty(),
                                   deadline: DateTime.UtcNow.AddSeconds(2));
            return response.Healthy;
        }
        catch
        {
            return false;
        }
    }
}

public sealed class GrpcServiceHealthChangedEventArgs(string address, bool isOnline) : EventArgs
{
    public string Address { get; } = address;
    public bool IsOnline { get; } = isOnline;
}