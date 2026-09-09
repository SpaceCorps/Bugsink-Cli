using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Issues;

/// <summary>
/// Covers all three mute endpoints. Which one is used follows from the options: a threshold means
/// "unmute once it gets noisy again", a period alone means "unmute after that long", neither means
/// "stay muted".
/// </summary>
public sealed class MuteIssueCommand : AsyncCommand<MuteIssueCommand.Settings>
{
    public sealed class Settings : SingleIssueSettings
    {
        [CommandOption("--for <N>")]
        [Description("Number of periods, used with --period")]
        public int? Periods { get; init; }

        [CommandOption("--period <PERIOD>")]
        [Description("year, month, week, day, hour or minute")]
        public string? Period { get; init; }

        [CommandOption("--threshold <N>")]
        [Description("Unmute once this many events arrive within the period, e.g. --threshold 10 --for 1 --period hour")]
        public int? Threshold { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();

        string action;
        object? body = null;

        if (settings.Threshold is not null)
        {
            if (settings.Periods is null || settings.Period is null)
                throw new BugsinkApiException("--threshold also needs --for <N> and --period <PERIOD>.");

            action = "mute-until/";
            body = new Dictionary<string, object?>
            {
                ["period_name"] = settings.Period,
                ["nr_of_periods"] = settings.Periods,
                ["gte_threshold"] = settings.Threshold
            };
        }
        else if (settings.Periods is not null || settings.Period is not null)
        {
            if (settings.Periods is null || settings.Period is null)
                throw new BugsinkApiException("--for and --period must be given together.");

            action = "mute-for/";
            body = new Dictionary<string, object?>
            {
                ["period_name"] = settings.Period,
                ["nr_of_periods"] = settings.Periods
            };
        }
        else
        {
            action = "mute/";
        }

        YamlOutput.Write(await client.PostAsync(settings.Path(action), body));
        Status.Ok($"Issue {settings.Issue} muted.");
        return 0;
    }
}
