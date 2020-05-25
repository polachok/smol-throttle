use smol;
use smol::Async;
use smol::Task;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Instant;

fn ping_pong_smol_write(count: u64) {
    use futures_util::io::{AsyncReadExt, AsyncWriteExt};
    smol::run(async {
        let pipe = Pipe::new(16).unwrap();
        let (a, b) = pipe.split();
        let mut a = Async::new(a).unwrap();
        let mut b = Async::new(b).unwrap();

        let write_task = async move {
            for _ in 0..count {
                let now = [0u8; 8];
                println!("{:?} written {:?}", Instant::now(), now);
                b.write(&now).await.unwrap();
            }
        };
        let read_task = async move {
            for _ in 0..count {
                let mut buf = [0u8; 8];
                a.read(&mut buf).await.unwrap();
                println!("{:?} read {:?}", Instant::now(), buf);
            }
        };
        let r = Task::local(read_task);
        let w = Task::local(write_task);
        r.await;
        w.await;
    });
}



fn main() {
    ping_pong_smol_write(10_000);    
}

struct Pipe(RawFd, RawFd);

impl Pipe {
    fn new(size: usize) -> io::Result<Self> {
        let mut pipefds: [i32; 2] = [0; 2];
        let rv = unsafe {
            libc::pipe2(
                &mut pipefds[0] as *mut _,
                libc::O_CLOEXEC | libc::O_NONBLOCK,
            )
        };
        if rv == -1 {
            return Err(io::Error::last_os_error());
        }
        //unix_io(|| unsafe { libc::fcntl(pipefds[0], libc::F_SETPIPE_SZ, 16) as isize })?;
        //unix_io(|| unsafe { libc::fcntl(pipefds[1], libc::F_SETPIPE_SZ, 16) as isize })?;
        println!(
            "{:?}",
            unix_io(|| unsafe { libc::fcntl(pipefds[1], libc::F_GETPIPE_SZ, 16) as isize })?
        );
        Ok(Pipe(pipefds[0], pipefds[1]))
    }
    fn split(self) -> (ReadEnd, WriteEnd) {
        (ReadEnd(self.0), WriteEnd(self.1))
    }
}

#[derive(Debug)]
struct WriteEnd(RawFd);

impl AsRawFd for WriteEnd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl Drop for WriteEnd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

impl io::Write for WriteEnd {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unix_io(|| unsafe {
            libc::write(
                self.as_raw_fd(),
                &buf[0] as *const u8 as *const core::ffi::c_void,
                buf.len(),
            )
        }).map(|x| x as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct ReadEnd(RawFd);

impl AsRawFd for ReadEnd {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

impl io::Read for ReadEnd {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unix_io(|| unsafe {
            libc::read(
                self.as_raw_fd(),
                &mut buf[0] as *mut u8 as *mut core::ffi::c_void,
                buf.len(),
            )
        }).map(|x| x as usize)
    }
}

impl Drop for ReadEnd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}

fn unix_io(mut op: impl FnMut() -> isize) -> Result<isize, io::Error> {
    match op() {
        -1 => Err(io::Error::last_os_error()),
        x => Ok(x),
    }
}


