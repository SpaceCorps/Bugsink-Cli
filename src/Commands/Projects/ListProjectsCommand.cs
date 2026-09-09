using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Projects;

public sealed class ListProjectsCommand : AsyncCommand<ListProjectsCommand.Settings>
{
    public sealed class Settings : PagedSettings
    {
        [CommandOption("--team <TEAM>")]
        [Description("Only list projects of this team (UUID or name)")]
        public string? Team { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        var path = "projects/";
        if (settings.Team is not null)
            path += $"?team={await client.ResolveTeamIdAsync(settings.Team)}";

        YamlOutput.Write(await client.GetPagedAsync(path, settings.EffectiveLimit));
        return 0;
    }
}
