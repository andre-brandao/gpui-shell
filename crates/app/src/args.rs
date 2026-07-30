//! Command-line argument parsing for GPUi Shell.

/// Command-line arguments.
pub struct Args {
    /// Optional input to prefill in the launcher.
    pub input: Option<String>,
    /// Parse the config files, report, and exit without starting the shell.
    pub validate: bool,
}

impl Args {
    /// Parse command-line arguments from `std::env::args()`.
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut input = None;
        let mut validate = false;

        let mut i = 1;
        while i < args.len() {
            if args[i] == "--input" || args[i] == "-i" {
                if i + 1 < args.len() {
                    input = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --input requires a value");
                    std::process::exit(1);
                }
            } else {
                validate |= args[i] == "--validate";
                i += 1;
            }
        }

        Args { input, validate }
    }
}
