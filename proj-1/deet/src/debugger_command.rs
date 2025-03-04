pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Continue,
    Backtrace,
    BreakPoint(String),
    Next,
    Print,
    Help,
    Exit,
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &[&str]) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            }
            "c" | "cont" | "continue" => Some(DebuggerCommand::Continue),
            "bt" | "back" | "backtrace" => Some(DebuggerCommand::Backtrace),
            "b" | "break" => Some(DebuggerCommand::BreakPoint(tokens[1].to_string())),
            "n" | "next" => Some(DebuggerCommand::Next),
            "p" | "print" => Some(DebuggerCommand::Print),
            "h" | "help" => Some(DebuggerCommand::Help),
            "e" | "exit" => Some(DebuggerCommand::Exit),
            _ => None,
        }
    }
}
