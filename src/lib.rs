//! ### TODO
//! 
//! * offer GC interface and example GCs
//! * rust currently doesn't allow generic static values - when it does, make this library 
//! truely generic: Add `read_pool`, `write_pool`. Keep `[Byte]Symbol`, `[Byte]SymbolPool` as
//! useful aliases.
//!

#![feature(hashmap_hasher)] 
#![feature(set_recovery)] 

extern crate fnv;
extern crate rustc_serialize;

use std::borrow::Borrow;
use std::collections::HashSet;
use std::collections::hash_state;
use std::fmt::{self, Debug, Display}; 
use std::hash::Hash;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData;
use std::ops::Deref;

use rustc_serialize::{Encodable, Decodable};

// ++++++++++++++++++++ Interned ++++++++++++++++++++

#[derive(Hash, PartialOrd, Ord)]
pub struct Interned<B: ?Sized, O>{
    inner: Arc<O>,
    _phantom: PhantomData<B>,
}

impl<B: ?Sized, O> Interned<B, O> {
    fn new(inner: Arc<O>) -> Self { Interned{ inner: inner, _phantom: PhantomData } }
}

impl<B: ?Sized, O> Clone for Interned<B, O> {
    fn clone(&self) -> Self { 
        Self::new(self.inner.clone())
    }
}

impl<B: ?Sized, O> PartialEq for Interned<B, O> {
    fn eq(&self, rhs: &Self) -> bool { 
        (&*self.inner) as *const _ as usize == (&*rhs.inner) as *const _ as usize
    }
}

impl<B: ?Sized, O> Eq for Interned<B, O> {}

impl<B: ?Sized, O> Borrow<B> for Interned<B, O> 
    where O: Borrow<B>    
{
    fn borrow(&self) -> &B { (*self.inner).borrow() }
}

//FIXME
/*impl<B: ?Sized, O> Into<O> for Interned<B, O> 
    where O: Clone
{
    fn into(self) -> O { self.inner.clone() }
}*/

impl<B: ?Sized, O> Debug for Interned<B, O> 
    where O: Debug
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (*self.inner).fmt(formatter)
    }
}

impl<B: ?Sized, O> Display for Interned<B, O> 
    where O: Display
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (*self.inner).fmt(formatter)
    }
}

impl<B: ?Sized, O> Deref for Interned<B, O> 
    where O: Borrow<B>
{
    type Target = B;
    fn deref(&self) -> &Self::Target { (*self.inner).borrow() }
}

impl<B: ?Sized, O> Encodable for Interned<B, O> 
    where O: Encodable
{
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> { 
        (*self.inner).encode(s)
    }
}

impl<B: ?Sized, O> Decodable for Interned<B, O> 
    where O: Decodable, Interned<B, O>: From<O>
{
    fn decode<D: rustc_serialize::Decoder>(d: &mut D) -> Result<Self, D::Error> { 
        Ok(Self::from(try!{O::decode(d)}))
    }
}

// ++++++++++++++++++++ InternPool ++++++++++++++++++++

pub struct InternPool<B: ?Sized, O> {
    data: HashSet<Interned<B, O>, hash_state::DefaultState<fnv::FnvHasher>>,
}

impl<B: ?Sized, O> InternPool<B, O> 
    where B: Hash + Eq, O: Borrow<B> + Hash + Eq
{
    fn new() -> Self {
        InternPool{ data: HashSet::default() }
    }

    pub fn get(&self, obj: &B) -> Option<&Interned<B, O>> {
        self.data.get(obj)
    }

    pub fn intern<X>(&mut self, obj: X) -> Interned<B, O>
        where X: Borrow<B>, X: Into<O>
    {
        match self.get(obj.borrow()) {
            Some(ret) => { return (*ret).clone(); }
            None => {}
        }
        let ret: Interned<B, O> = Interned::new(Arc::new(obj.into()));
        self.data.insert(ret.clone());
        ret
    }
}

// ++++++++++++++++++++ Symbol, SymbolPool ++++++++++++++++++++

pub type Symbol = Interned<str, String>;
pub type SymbolPool = InternPool<str, String>;

fn symbol_pool_instance() -> &'static RwLock<SymbolPool> {
    static POOL: Option<RwLock<SymbolPool>> = None;

    match &POOL {
        &Some(ref r) => r,

        // beware of racey hack!
        pool => unsafe {
            let init = Some(RwLock::new(SymbolPool::new()));
            ::std::ptr::write(pool as *const _ as *mut Option<RwLock<SymbolPool>>, init);
            POOL.as_ref().unwrap()
        }
    }
}

pub fn read_symbol_pool() -> RwLockReadGuard<'static, SymbolPool> {
    symbol_pool_instance().read().unwrap()
}

pub fn write_symbol_pool() -> RwLockWriteGuard<'static, SymbolPool> {
    symbol_pool_instance().write().unwrap()
}

impl<'a> From<&'a str> for Symbol {
    fn from(s: &'a str) -> Symbol {
        write_symbol_pool().intern(s)
    }
}

impl<'a> From<String> for Symbol {
    fn from(s: String) -> Symbol {
        write_symbol_pool().intern(s)
    }
}

// ++++++++++++++++++++ ByteSymbol, ByteSymbolPool ++++++++++++++++++++

pub type ByteSymbol = Interned<[u8], Vec<u8>>;
pub type ByteSymbolPool = InternPool<[u8], Vec<u8>>;

fn byte_symbol_pool_instance() -> &'static RwLock<ByteSymbolPool> {
    static POOL: Option<RwLock<ByteSymbolPool>> = None;

    match &POOL {
        &Some(ref r) => r,

        // beware of racey hack!
        pool => unsafe {
            let init = Some(RwLock::new(ByteSymbolPool::new()));
            ::std::ptr::write(pool as *const _ as *mut Option<RwLock<ByteSymbolPool>>, init);
            POOL.as_ref().unwrap()
        }
    }
}

pub fn read_byte_symbol_pool() -> RwLockReadGuard<'static, ByteSymbolPool> {
    byte_symbol_pool_instance().read().unwrap()
}

pub fn write_byte_symbol_pool() -> RwLockWriteGuard<'static, ByteSymbolPool> {
    byte_symbol_pool_instance().write().unwrap()
}

impl<'a> From<&'a [u8]> for ByteSymbol {
    fn from(s: &'a [u8]) -> ByteSymbol {
        write_byte_symbol_pool().intern(s)
    }
}

impl<'a> From<Vec<u8>> for ByteSymbol {
    fn from(s: Vec<u8>) -> ByteSymbol {
        write_byte_symbol_pool().intern(s)
    }
}




