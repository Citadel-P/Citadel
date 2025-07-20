namespace Domain.Contracts.Resources.Volumes;

public record VolumePublishStatus(string NodeID, string State, IReadOnlyDictionary<string, string> PublishContext);
