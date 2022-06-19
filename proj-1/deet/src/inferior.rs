use crate::dwarf_data::DwarfData;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::sys::{ptrace, signal::Signal};
use nix::unistd::Pid;
use std::{collections::HashMap, mem::size_of};
use std::{
    io,
    os::unix::prelude::CommandExt,
    process::{Child, Command},
};

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "ptrace TRACEME failed"))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(
        target: &str,
        args: &Vec<String>,
        break_points: &mut HashMap<usize, u8>,
    ) -> Option<Inferior> {
        let mut cmd = Command::new(target);
        cmd.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        let child = cmd.spawn().ok()?;
        let mut inferior = Inferior { child };
        let bp_copy = break_points.clone();
        for addr in bp_copy.keys() {
            match inferior.write_byte(*addr, 0xcc) {
                Ok(orig_mode) => {
                    // println!("Set breakpoint at {:#x}", addr);
                    break_points.insert(*addr, orig_mode);
                }
                Err(_) => {
                    println!("Could not set breakpoint at {:#x}", addr);
                }
            }
        }

        Some(inferior)
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_exec(&self, signal: Option<Signal>) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), signal)?;
        self.wait(None)
    }
    // pub fn continue_bp(&self) -> Result<Status, nix::Error> {
    //     ptrace::step(self.pid(), Signal::SIGTRAP);
    // }
    pub fn kill(&mut self) -> io::Result<()> {
        println!("Killing running inferior (pid {})", self.pid());
        self.child.kill()
    }
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid())?;
        let mut rip = regs.rip as usize;
        let mut rbp = regs.rbp as usize;
        loop {
            let file = debug_data.get_line_from_addr(rip);
            let func = debug_data.get_function_from_addr(rip);

            match (&file, &func) {
                (Some(file), Some(func)) => {
                    println!("{} ({})", func, file);
                }
                (_, _) => {
                    println!("%rip register: {:#x}", rip);
                }
            }
            if let Some(func) = func {
                if func == "main" {
                    break;
                }
            }
            rip = ptrace::read(self.pid(), (rbp + 8) as ptrace::AddressType)? as usize;
            rbp = ptrace::read(self.pid(), rbp as ptrace::AddressType)? as usize;
        }

        Ok(())
    }

    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_mode = (word >> (8 * byte_offset)) & 0xff;
        let masked_word = word & !(0xff << (8 * byte_offset));
        let updated_word = masked_word | ((val as u64) << (8 * byte_offset));
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_mode as u8)
    }
}
