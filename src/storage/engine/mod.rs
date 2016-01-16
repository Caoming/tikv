use self::memory::EngineBtree;
use std::{error, result};
use std::fmt::{self, Display, Formatter};
use self::rocksdb::EngineRocksdb;

mod memory;
mod rocksdb;

#[derive(Debug)]
pub enum Modify<'a> {
    Delete(&'a [u8]),
    Put((&'a [u8], &'a [u8])),
}

pub trait Engine {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn seek(&self, key: &[u8]) -> Result<Option<(Vec<u8>, Vec<u8>)>>;
    fn write(&mut self, batch: Vec<Modify>) -> Result<()>;

    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.write(vec![Modify::Put((key, value))])
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.write(vec![Modify::Delete(key)])
    }
}

#[derive(Debug)]
pub enum Dsn<'a> {
    Memory,
    RocksDBPath(&'a str),
}

pub fn new_engine(desc: Dsn) -> Result<Box<Engine>> {
    match desc {
        Dsn::Memory => Ok(Box::new(EngineBtree::new())),
        Dsn::RocksDBPath(path) => {
            EngineRocksdb::new(path).map(|engine| -> Box<Engine> { Box::new(engine) })
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Other(Box<error::Error + Send + Sync>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            Error::Other(ref error) => Display::fmt(error, f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            &Error::Other(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &Error::Other(ref e) => e.cause(),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::{Dsn, Engine, Modify};

    #[test]
    fn memory() {
        let mut e = super::new_engine(Dsn::Memory).unwrap();
        get_put(e.as_mut());
        batch(e.as_mut());
        seek(e.as_mut());
    }

    #[test]
    fn rocksdb() {
        let mut e = super::new_engine(Dsn::RocksDBPath("/tmp/rocks")).unwrap();
        get_put(e.as_mut());
        batch(e.as_mut());
        seek(e.as_mut());
    }

    fn assert_has<T: Engine + ?Sized>(engine: &T, key: &[u8], value: &[u8]) {
        assert_eq!(engine.get(key).unwrap().unwrap(), value);
    }

    fn assert_none<T: Engine + ?Sized>(engine: &T, key: &[u8]) {
        assert_eq!(engine.get(key).unwrap(), None);
    }

    fn assert_seek<T: Engine + ?Sized>(engine: &T, key: &[u8], pair: (&[u8], &[u8])) {
        let (k, v) = engine.seek(key).unwrap().unwrap();
        assert_eq!(k, pair.0);
        assert_eq!(v, pair.1);
    }

    fn get_put<T: Engine + ?Sized>(engine: &mut T) {
        assert_none(engine, b"x");
        engine.put(b"x", b"1").unwrap();
        assert_has(engine, b"x", b"1");
        engine.put(b"x", b"2").unwrap();
        assert_has(engine, b"x", b"2");
        engine.delete(b"x").unwrap();
        assert_none(engine, b"x");
    }

    fn batch<T: Engine + ?Sized>(engine: &mut T) {
        engine.write(vec![Modify::Put((b"x", b"1")), Modify::Put((b"y", b"2"))]).unwrap();
        assert_has(engine, b"x", b"1");
        assert_has(engine, b"y", b"2");

        engine.write(vec![Modify::Delete(b"x"), Modify::Delete(b"y")]).unwrap();
        assert_none(engine, b"y");
        assert_none(engine, b"y");
    }

    fn seek<T: Engine + ?Sized>(engine: &mut T) {
        engine.put(b"x", b"1").unwrap();
        assert_seek(engine, b"x", (b"x", b"1"));
        assert_seek(engine, b"a", (b"x", b"1"));
        engine.put(b"z", b"2").unwrap();
        assert_seek(engine, b"y", (b"z", b"2"));
        assert_seek(engine, b"x\x00", (b"z", b"2"));
        assert_eq!(engine.seek(b"z\x00").unwrap(), None);
        engine.delete(b"x").unwrap();
        engine.delete(b"z").unwrap();
    }
}