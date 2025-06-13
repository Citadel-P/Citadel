namespace Domain.Contracts.Resources.Volumes;

public record VolumeCapacityRange(long? RequiredBytes, long? LimitBytes);
