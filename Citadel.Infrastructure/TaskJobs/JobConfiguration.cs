namespace Infrastructure.TaskJobs;

public class JobConfiguration
{
    public double SystemInfoInterval { get; set; } = 15;

    public double ContainersInfoInterval { get; set; } = 5;
}
