namespace Tests.Acceptance.Infrastructure;

internal static class CandidateImageTestEnvironment
{
    public const string RequireImagesVariable =
        "CITADEL_ACCEPTANCE_REQUIRE_CANDIDATE_IMAGES";

    public static bool ShouldRun(params string[] imageVariables) =>
        RequiresCandidateImages ||
        imageVariables.All(
            variable => !string.IsNullOrWhiteSpace(
                Environment.GetEnvironmentVariable(variable)));

    public static string GetRequiredImage(string variable)
    {
        var image = Environment.GetEnvironmentVariable(variable);
        Assert.False(
            string.IsNullOrWhiteSpace(image),
            $"Set {variable} to the candidate image under test.");
        return image!;
    }

    private static bool RequiresCandidateImages =>
        string.Equals(
            Environment.GetEnvironmentVariable(RequireImagesVariable),
            bool.TrueString,
            StringComparison.OrdinalIgnoreCase);
}
