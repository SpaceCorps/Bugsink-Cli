using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Projects;

public sealed class CreateProjectCommand : AsyncCommand<CreateProjectCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandOption("--team <TEAM>")]
        [Description("Owning team, by UUID or name")]
        public required string Team { get; init; }

        [CommandOption("--name <NAME>")]
        [Description("Project name")]
        public required string Name { get; init; }

        [CommandOption("--visibility <VISIBILITY>")]
        [Description("joinable, discoverable or team_members")]
        public string? Visibility { get; init; }

        [CommandOption("--alert-on-new-issue <BOOL>")]
        [Description("Send an alert the first time an issue is seen")]
        public bool? AlertOnNewIssue { get; init; }

        [CommandOption("--alert-on-regression <BOOL>")]
        [Description("Send an alert when a resolved issue comes back")]
        public bool? AlertOnRegression { get; init; }

        [CommandOption("--alert-on-unmute <BOOL>")]
        [Description("Send an alert when a muted issue unmutes itself")]
        public bool? AlertOnUnmute { get; init; }

        [CommandOption("--retention <N>")]
        [Description("Maximum number of events to keep for this project")]
        public long? RetentionMaxEventCount { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        var body = new Dictionary<string, object?>
        {
            ["team"] = await client.ResolveTeamIdAsync(settings.Team),
            ["name"] = settings.Name
        };
        if (settings.Visibility is not null) body["visibility"] = settings.Visibility;
        if (settings.AlertOnNewIssue is not null) body["alert_on_new_issue"] = settings.AlertOnNewIssue;
        if (settings.AlertOnRegression is not null) body["alert_on_regression"] = settings.AlertOnRegression;
        if (settings.AlertOnUnmute is not null) body["alert_on_unmute"] = settings.AlertOnUnmute;
        if (settings.RetentionMaxEventCount is not null)
            body["retention_max_event_count"] = settings.RetentionMaxEventCount;

        YamlOutput.Write(await client.PostAsync("projects/", body));
        Status.Ok($"Project {settings.Name} created.");
        return 0;
    }
}
