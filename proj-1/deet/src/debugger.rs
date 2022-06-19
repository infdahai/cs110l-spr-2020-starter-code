use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
};

use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{Inferior, Status};
use nix::sys::signal::Signal;
use rustyline::error::ReadlineError;
use rustyline::Editor;
pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    break_point: HashMap<usize, u8>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        debug_data.print();

        println!("Type 'h' or 'help' for a list of commands.");

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            break_point: HashMap::new(),
        }
    }
    pub fn print_line_code(
        &self,
        file: &str,
        num: &usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cnt = *num;
        let data = fs::File::open(file)?;
        let reader = BufReader::new(data);
        for line in reader.lines() {
            cnt -= 1;
            if cnt == 0 {
                println!("line: {} \ncode: {}", num, line?);
                break;
            }
        }
        Ok(())
    }
    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if self.inferior.is_some() {
                        self.inferior.as_mut().unwrap().kill().unwrap();
                        self.inferior = None;
                    }
                    if let Some(inferior) =
                        Inferior::new(&self.target, &args, &mut self.break_point)
                    {
                        self.inferior = Some(inferior);
                        match self.inferior.as_mut().unwrap().continue_exec(None).unwrap() {
                            Status::Exited(exit_code) => {
                                println!("Child exited (status {}) ", exit_code);
                                self.inferior = None;
                            }
                            Status::Signaled(signal) => {
                                println!("Child exited due to signal {}", signal);
                                self.inferior = None;
                            }
                            Status::Stopped(signal, rip) => {
                                println!(
                                    "Child stopped due to signal {} at address {}",
                                    signal, rip
                                );
                                let file = self.debug_data.get_line_from_addr(rip);
                                let func = self.debug_data.get_function_from_addr(rip);
                                if file.is_some() && func.is_some() {
                                    let file = file.unwrap();
                                    println!("Stopped at {}({})", func.unwrap(), file);
                                    let path = file.file;
                                    let number = file.number;
                                    self.print_line_code(&path, &number).unwrap();
                                }
                            }
                        }
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Continue => {
                    if let Some(inferior) = &mut self.inferior {
                        match inferior.continue_exec(None).unwrap() {
                            Status::Exited(exit_code) => {
                                println!("Child exited (status {}) ", exit_code);
                                self.inferior = None;
                            }
                            Status::Signaled(signal) => {
                                println!("Child exited due to signal {}", signal);
                                self.inferior = None;
                            }
                            Status::Stopped(signal, rip) => {
                                match signal {
                                    Signal::SIGTRAP => (),

                                    _ => (),
                                }
                                println!(
                                    "Child stopped due to signal {} at address {}",
                                    signal, rip
                                );
                                let file = self.debug_data.get_line_from_addr(rip);
                                let func = self.debug_data.get_function_from_addr(rip);
                                if file.is_some() && func.is_some() {
                                    let file = file.unwrap();
                                    println!("Stopped at {}({})", func.unwrap(), file);
                                    let path = file.file;
                                    let number = file.number;
                                    self.print_line_code(&path, &number).unwrap();
                                }
                            }
                        }
                    } else {
                        println!("No inferior to continue");
                    }
                }
                DebuggerCommand::Backtrace => {
                    if self.inferior.is_none() {
                        println!("No inferior to backtrace");
                    } else {
                        let inferior = self.inferior.as_ref().unwrap();
                        inferior.print_backtrace(&self.debug_data).unwrap();
                    }
                }
                DebuggerCommand::BreakPoint(addr) => {
                    let break_point_addr;
                    if let Some(addr_) = addr.strip_prefix("*") {
                        if let Some(addr) = self.parse_addr(addr_) {
                            break_point_addr = addr;
                        } else {
                            println!("Invalid address");
                            continue;
                        }
                    } else if let Ok(line_num) = addr.parse::<usize>() {
                        if let Some(line_addr) = self.debug_data.get_addr_for_line(None, line_num) {
                            break_point_addr = line_addr;
                        } else {
                            println!("Could not find address for line {}", line_num);
                            continue;
                        }
                    } else if let Some(func_addr) =
                        self.debug_data.get_addr_for_function(None, &addr)
                    {
                        break_point_addr = func_addr;
                    } else {
                        println!("Could not find address for correct usage {}", addr);
                        continue;
                    }
                    if self.inferior.is_some() {
                        if let Ok(inst) = self
                            .inferior
                            .as_mut()
                            .unwrap()
                            .write_byte(break_point_addr, 0xcc)
                        {
                            println!(
                                "Set breakpoint {} at {:#x}",
                                self.break_point.len(),
                                break_point_addr
                            );
                            self.break_point.insert(break_point_addr, inst);
                        } else {
                            println!("Invalid break point address {:#x}", break_point_addr);
                        }
                    } else {
                        println!(
                            "Set breakpoint {} at {:#x}",
                            self.break_point.len(),
                            break_point_addr
                        );
                        self.break_point.insert(break_point_addr, 0);
                    }
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        self.inferior.as_mut().unwrap().kill().unwrap();
                        self.inferior = None;
                    }
                }
                DebuggerCommand::Exit => {
                    if self.inferior.is_some() {
                        self.inferior.as_mut().unwrap().kill().unwrap();
                        self.inferior = None;
                    }
                    println!("Exiting debugger");
                    return;
                }
                DebuggerCommand::Next => {}
                DebuggerCommand::Print => {}
                DebuggerCommand::Help => {
                    println!("h | help - ask for help");
                    println!("r | run - run new program");
                    println!("c | cont | continue - continue code execution");
                    println!("b | break - set a breakpoint");
                    println!("n | next - single step execution");
                    println!("p | print - print the variables");
                    println!("q | quit - quit the program");
                    println!("e | exit - quit the debugger");
                    println!("Some command is unimplmented.{{list}}");
                }
            }
        }
    }

    fn parse_addr(&self, addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"exit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
