using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Teams;

public sealed class GetTeamCommand : AsyncCommand<GetTeamCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<TEAM>")]
        [Description("Team UUID or name")]
        public required string Team { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var teamId = await client.ResolveTeamIdAsync(settings.Team);
        YamlOutput.Write(await client.GetAsync($"teams/{teamId}/"));
        return 0;
    }
}
