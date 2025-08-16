use nix::libc::{siginfo_t, ucontext_t};
use nix::sys::signal;

struct XServerNotifier(AtomicBool);

extern "C" fn signal_to_flag(_: i32, _: *mut siginfo_t, ctx: *mut c_void) {}

impl XServerNotifier {
    fn new() -> Self {
        let flag = AtomicBool::new(false);

        unsafe {
            signal::signal(
                signal::SIGUSR1,
                signal::SigHandler::SigAction(signal_to_flag),
            )
        };

        Self(flag)
    }

    unsafe fn setup(command: &mut Command) -> &mut Command {
        unsafe {
            command.pre_exec(|| {
                signal::signal(signal::SIGUSR1, signal::SigHandler::SigIgn)?;
                Ok(())
            })
        }
    }

    fn wait(&self) {
        todo!()
    }
}
