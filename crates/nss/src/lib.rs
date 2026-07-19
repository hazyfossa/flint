pub use libc::passwd;
use libc::{gid_t, uid_t};

use std::{
    ffi::{CStr, OsStr, OsString, c_char},
    io::{self, ErrorKind},
    mem,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    ptr,
};

unsafe fn getpwnam(username: &str) -> io::Result<Option<passwd>> {
    let arg = CStr::from_bytes_until_nul(username.as_bytes())
        .map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;

    let mut mem_ret = mem::MaybeUninit::<passwd>::uninit();
    let mut mem_aux = vec![0; 2048];
    let mut ptr_ret = ptr::null_mut::<passwd>();

    let status: io::Result<()> = loop {
        let s = unsafe {
            libc::getpwnam_r(
                arg.as_ptr(),
                mem_ret.as_mut_ptr(),
                mem_aux.as_mut_ptr(),
                mem_aux.len(),
                &mut ptr_ret,
            )
        };

        match s {
            libc::ERANGE => {
                let newsize = mem_aux
                    .len()
                    .checked_mul(2)
                    .expect("overflow: libc expects an unreasonable amount of memory");
                mem_aux.resize(newsize, 0);
                continue;
            }

            0 => break Ok(()),

            err => break Err(io::Error::from_raw_os_error(err)),
        }
    };

    status?;

    Ok(match ptr_ret.is_null() {
        true => None, // we checked for errors with `status` above, so null means "not found"
        false => Some(unsafe { ptr_ret.read() }),
    })
}

unsafe fn raw_read<'a, T>(p: *const c_char) -> T
where
    T: From<&'a OsStr>,
{
    let cstr = unsafe { CStr::from_ptr(p).to_bytes() };
    T::from(OsStr::from_bytes(cstr))
}

// TODO: properly read all of passwd here

pub struct UserDefinition {
    pub uid: uid_t,
    pub gid: gid_t,
    pub home: PathBuf,
    pub shell: OsString,
}

pub fn resolve(name: &str) -> io::Result<Option<UserDefinition>> {
    unsafe {
        Ok(getpwnam(name)?.map(|p| UserDefinition {
            uid: p.pw_uid,
            gid: p.pw_gid,
            home: raw_read(p.pw_dir),
            shell: raw_read(p.pw_shell),
        }))
    }
}
