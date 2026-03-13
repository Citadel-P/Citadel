namespace Domain.Contracts.Resources.Images;

public record PullImageStreamItem(
    string? Id = null,
    string? From = null,
    string? Stream = null,
    string? Status = null,
    string? ErrorMessage = null,
    string? ProgressMessage = null,
    string? DockerImageId = null,
    string? Digest = null,
    ImagePullProgress? Progress = null,
    ImagePullError? Error = null
);

public record ImagePullProgress(
    string? Units,
    long? Current,
    long? Total,
    long? Start
);

public record ImagePullError(long? Code, string? Message);

