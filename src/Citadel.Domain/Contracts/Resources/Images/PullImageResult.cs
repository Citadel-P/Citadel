namespace Domain.Contracts.Resources.Images;

public record PullImageResult(
    string? Id,
    string? From,
    string? Stream,
    string? Status,
    string? ErrorMessage,
    string? ProgressMessage,
    ImagePullProgress? Progress,
    ImagePullError? Error
);

public record ImagePullProgress(
    string? Units,
    long? Current,
    long? Total,
    long? Start
);

public record ImagePullError(long? Code, string? Message);

