namespace Domain.Contracts.Resources.Images;

public record PullImageResult(
    string? Id,
    string? From,
    string? Stream,
    string? Status,
    string? ErrorMessage,
    string? ProgressMessage,
    PullImageJsonProgress? Progress,
    PullImageJsonError? Error
);

public record PullImageJsonProgress(
    string? Units,
    long? Current,
    long? Total,
    long? Start
);

public record PullImageJsonError(long? Code, string? Message);

