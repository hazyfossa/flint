use libc::passwd;

use super::*;
use std::{
    ffi::CStr,
    io::{self, ErrorKind},
    mem, ptr,
};

fn getpwnam(username: &str) -> io::Result<Option<passwd>> {
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

pub struct NSS;
impl UserProvider for NSS {
    type Error = io::Error;
    async fn resolve(&mut self, name: &str) -> Result<Option<UserMeta>, Self::Error> {
        Ok(getpwnam(name)?.map(|p| UserMeta {
            uid: p.pw_uid,
            gid: p.pw_gid,
        }))
    }
}
