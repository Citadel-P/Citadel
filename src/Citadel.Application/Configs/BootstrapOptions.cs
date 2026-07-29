namespace Application.Configs;

public sealed class BootstrapOptions
{
    public const string SectionName = "Bootstrap";

    public string? AdminName { get; set; }
    public string? AdminEmail { get; set; }
    public string? AdminPasswordFile { get; set; }
}
