use argh::FromArgs;

#[derive(FromArgs, Debug)]
/// Smart window launcher/switcher for Hyprland.
pub struct Args {
    /// force launch a new instance
    #[argh(switch, short = 'n')]
    pub new: bool,

    /// command to run (and arguments)
    #[argh(positional, greedy)]
    pub command: Vec<String>,
}