using Domain;
using Domain.Entities.Alerts;
using Domain.Entities.Deployments;
using Domain.Entities.Stacks;

namespace Application.Services;

internal sealed record DeploymentUpdateEvaluation(
    AutoUpdateState State,
    bool UpdateAvailable);

internal sealed class DeploymentUpdateEvaluator
{
    public DeploymentUpdateEvaluation Evaluate(
        string currentDigest,
        string remoteDigest,
        DateTime checkedAt)
    {
        var updateAvailable = !string.Equals(
            currentDigest,
            remoteDigest,
            StringComparison.OrdinalIgnoreCase);

        return new DeploymentUpdateEvaluation(
            new AutoUpdateState(
                checkedAt,
                updateAvailable ? AutoUpdateStatus.UpdateAvailable : AutoUpdateStatus.UpToDate,
                currentDigest,
                remoteDigest),
            updateAvailable);
    }
}

internal sealed record ManualStackUpdateEvaluation(
    ManualStackUpdateState State,
    IReadOnlyList<StackImageUpdateItem> AvailableUpdates,
    IReadOnlyList<StackImageUpdateItem> NewlyDetectedUpdates,
    int BaselinesCreated);

internal sealed class ManualStackUpdateEvaluator
{
    public ManualStackUpdateEvaluation Evaluate(
        StackUpdateState previousState,
        IReadOnlyList<ManualStackImageCheck> checks,
        IReadOnlyDictionary<ImageKey, string> digests,
        DateTime checkedAt)
    {
        var previousStates = (previousState as ManualStackUpdateState)?
            .RecreateStackOnNewImageState.AutoUpdateStates
            .GroupBy(
                state => StateKey(state.ServiceName, state.ImageName),
                StringComparer.OrdinalIgnoreCase)
            .ToDictionary(
                group => group.Key,
                group => group.Last(),
                StringComparer.OrdinalIgnoreCase)
            ?? new Dictionary<string, ImageUpdateState>(StringComparer.OrdinalIgnoreCase);

        var nextStates = new List<ImageUpdateState>();
        var availableUpdates = new List<StackImageUpdateItem>();
        var newlyDetectedUpdates = new List<StackImageUpdateItem>();
        var baselinesCreated = 0;

        foreach (var check in checks)
        {
            if (!digests.TryGetValue(check.Key, out var remoteDigest)
                || string.IsNullOrWhiteSpace(remoteDigest))
            {
                continue;
            }

            var key = StateKey(check.ServiceName, check.ImageName);
            previousStates.TryGetValue(key, out var previous);
            var hasBaseline = !string.IsNullOrWhiteSpace(previous?.CurrentDigest);
            var currentDigest = hasBaseline ? previous!.CurrentDigest : remoteDigest;
            if (!hasBaseline)
            {
                baselinesCreated++;
            }

            var updateAvailable = !string.Equals(
                currentDigest,
                remoteDigest,
                StringComparison.OrdinalIgnoreCase);
            nextStates.Add(new ImageUpdateState(
                check.ServiceName,
                check.ImageName,
                currentDigest,
                remoteDigest,
                checkedAt,
                updateAvailable));

            if (!updateAvailable)
            {
                continue;
            }

            var update = new StackImageUpdateItem(
                check.ServiceName,
                check.ImageName,
                currentDigest,
                remoteDigest);
            availableUpdates.Add(update);

            var alreadyReported = previous is { UpdateAvailable: true, RemoteDigest: not null }
                && string.Equals(previous.RemoteDigest, remoteDigest, StringComparison.OrdinalIgnoreCase);
            if (!alreadyReported)
            {
                newlyDetectedUpdates.Add(update);
            }
        }

        return new ManualStackUpdateEvaluation(
            new ManualStackUpdateState(new RecreateStackOnNewImageState(nextStates)),
            availableUpdates,
            newlyDetectedUpdates,
            baselinesCreated);
    }

    internal static string StateKey(string serviceName, string imageName)
        => $"{serviceName}\n{imageName}";
}
