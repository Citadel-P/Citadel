using Domain.Entities.Identity;
using Infrastructure.EntityFramework.Seed;
using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Metadata.Builders;

namespace Infrastructure.EntityFramework.Configurations;

internal sealed class UserTeamConfiguration : IEntityTypeConfiguration<UserTeam>
{
    public void Configure(EntityTypeBuilder<UserTeam> builder)
    {
        builder.HasKey(e => new { e.UserId, e.TeamId });

        builder.HasData(DbSeed.UsersTeams);
    }
}