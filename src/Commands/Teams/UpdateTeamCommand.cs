using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Teams;

public sealed class UpdateTeamCommand : AsyncCommand<UpdateTeamCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<TEAM>")]
        [Description("Team UUID or name")]
        public required string Team { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("New team name")]
        public string? Name { get; init; }

        [CommandOption("--visibility <VISIBILITY>")]
        [Description("joinable, discoverable or hidden")]
        public string? Visibility { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var body = new Dictionary<string, object?>();
        if (settings.Name is not null) body["name"] = settings.Name;
        if (settings.Visibility is not null) body["visibility"] = settings.Visibility;

        if (body.Count == 0)
            throw new BugsinkApiException("Nothing to update. Pass --name and/or --visibility.");

        var client = settings.CreateClient();
        var teamId = await client.ResolveTeamIdAsync(settings.Team);

        YamlOutput.Write(await client.PatchAsync($"teams/{teamId}/", body));
        Status.Ok($"Team {settings.Team} updated.");
        return 0;
    }
}
