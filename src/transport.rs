use std::io::{self, Read, Write};
use std::net::SocketAddr;

use bytes::Buf;
use futures::Poll;
use hyper::server::conn::AddrStream;
use tokio_io::{AsyncRead, AsyncWrite};

pub trait Transport: AsyncRead + AsyncWrite {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl Transport for AddrStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr())
    }
}

pub(crate) struct LiftIo<T>(pub(crate) T);

impl<T: Read> Read for LiftIo<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl<T: Write> Write for LiftIo<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<T: AsyncRead> AsyncRead for LiftIo<T> {}

impl<T: AsyncWrite> AsyncWrite for LiftIo<T> {
    fn write_buf<B: Buf>(&mut self, buf: &mut B) -> Poll<usize, io::Error> {
        self.0.write_buf(buf)
    }

    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.0.shutdown()
    }
}

impl<T: AsyncRead + AsyncWrite> Transport for LiftIo<T> {
    fn remote_addr(&self) -> Option<SocketAddr> {
        None
    }
}
