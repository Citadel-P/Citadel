using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Quartz;

namespace Infrastructure.TaskJobs;

internal static class Extensions
{
    public static IServiceCollection AddTaskJobs(this IServiceCollection services, IConfiguration configuration)
    {
        var jobConfig = configuration.GetSection(nameof(JobConfiguration)).Get<JobConfiguration>();

        services.AddQuartz(q =>
        {
            // Add SystemInfo job
            q.AddJob<PlatformInfoJob>(opts => opts.WithIdentity(PlatformInfoJob.JobKey));
            q.AddTrigger(opts => opts
                .ForJob(PlatformInfoJob.JobKey)
                .StartNow()
                 .WithSimpleSchedule(x => x
                    .WithInterval(TimeSpan.FromSeconds(jobConfig.SystemInfoInterval))
                    .RepeatForever())
            );

            // Add ContainersInfo job
            q.AddJob<ContainersInfoJob>(opts => opts.WithIdentity(ContainersInfoJob.JobKey));
            q.AddTrigger(opts => opts
                .ForJob(ContainersInfoJob.JobKey)
                .StartNow()
                 .WithSimpleSchedule(x => x
                    .WithInterval(TimeSpan.FromSeconds(jobConfig.ContainersInfoInterval))
                    .RepeatForever())
            );

            // Add DaemonEvent job
            q.AddJob<DaemonEventJob>(opts => opts.WithIdentity(DaemonEventJob.JobKey));
            q.AddTrigger(opts => opts
                .ForJob(DaemonEventJob.JobKey)
                .StartNow()
            );

            // Add LogCleanup job
            q.AddJob<LogCleanupJob>(opts => opts.WithIdentity(LogCleanupJob.JobKey));
            q.AddTrigger(opts => opts
                .ForJob(LogCleanupJob.JobKey)
                .WithCronSchedule("0 0 0 * * ?") // Runs daily at midnight
            );
        });

        services.AddQuartzHostedService(options =>
        {
            options.WaitForJobsToComplete = true;
            options.AwaitApplicationStarted = true;
        });

        return services;
    }
}