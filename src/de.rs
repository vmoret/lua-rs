//! Lua deserialization.
use std::fmt;

use num_traits::{cast::cast, NumCast};

use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::Deserialize;

use crate::{
    ffi,
    state::{self, State},
    lref::LRef,
};
struct Deserializer<'de> {
    state: &'de State,
}

impl<'de> Deserializer<'de> {
    fn new(state: &'de State) -> Self {
        Deserializer { state }
    }

    fn parse_i64(&self) -> Option<i64> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tointegerx(self.state.as_ptr(), -1, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            Some(n)
        }
    }

    fn parse_integer<T: NumCast>(&self) -> Result<T, Error> {
        match self.parse_i64() {
            Some(n) => cast(n).ok_or_else(|| {
                error!("failed to cast {} from i64", n);
                Error::new(format!("failed to cast {} from i64", n))
            }),
            None => {
                error!("unable to deserialize as integer");
                Err(Error::new("unable to deserialize as integer"))
            },
        }
    }

    fn parse_f64(&self) -> Option<f64> {
        let mut isnum = 0;
        let n = unsafe { ffi::lua_tonumberx(self.state.as_ptr(), -1, &mut isnum) };
        if isnum == 0 {
            None
        } else {
            Some(n)
        }
    }

    fn parse_float<T: NumCast>(&self) -> Result<T, Error> {
        match self.parse_f64() {
            Some(n) => cast(n).ok_or_else(|| {
                error!("failed to cast {} from f64", n);
                Error::new(format!("failed to cast {} from f64", n))
            }),
            None => {
                error!("unable to deserialize as float");
                Err(Error::new("unable to deserialize as float"))
            },
        }
    }

    fn parse_bytes(&self) -> Option<&'de [u8]> {
        let mut len = 0;
        unsafe {
            let ptr = ffi::lua_tolstring(self.state.as_ptr(), -1, &mut len);
            if ptr.is_null() {
                None
            } else {
                let data = ptr as *const u8;
                Some(std::slice::from_raw_parts(data, len))
            }
        }
    }

    fn parse_str(&self) -> Option<&'de str> {
        self.parse_bytes()
            .map(|v| std::str::from_utf8(v).ok())
            .flatten()
    }
}

impl State {
    pub fn deserialize<'de, T>(&'de self) -> Result<T, Error>
    where
        T: Deserialize<'de>,
    {
        let mut deserializer = Deserializer::new(self);
        T::deserialize(&mut deserializer)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let typ = self.state.value_type(-1);
        debug!(
            "deserialize_any() top = {}, type = {}",
            self.state.get_top(),
            typ
        );

        match typ {
            state::LUA_TBOOLEAN => self.deserialize_bool(visitor),
            state::LUA_TFUNCTION => {
                error!("unable to deserialize functions");
                Err(Error::new("unable to deserialize functions"))
            },
            state::LUA_TLIGHTUSERDATA => {
                error!("unable to deserialize light user data");
                Err(Error::new("unable to deserialize light user data"))
            },
            state::LUA_TNIL => self.deserialize_unit(visitor),
            state::LUA_TNUMBER => unsafe {
                if ffi::lua_isinteger(self.state.as_ptr(), -1) != 0 {
                    self.deserialize_i64(visitor)
                } else {
                    self.deserialize_f64(visitor)
                }
            },
            state::LUA_TSTRING => self.deserialize_bytes(visitor),
            state::LUA_TTABLE => {
                todo!()
            }
            state::LUA_TTHREAD => {
                error!("unable to deserialize threads");
                Err(Error::new("unable to deserialize threads"))
            },
            state::LUA_TUSERDATA => {
                error!("unable to deserialize user data");
                Err(Error::new("unable to deserialize user data"))
            },
            _ => {
                error!("invalid value type");
                Err(Error::new("invalid value type"))
            },
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_bool()");
        let v = unsafe { ffi::lua_toboolean(self.state.as_ptr(), -1) != 0 };
        visitor.visit_bool(v)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_i8()");
        let v = self.parse_integer()?;
        visitor.visit_i8(v)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_i16()");
        let v = self.parse_integer()?;
        visitor.visit_i16(v)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_i32()");
        let v = self.parse_integer()?;
        visitor.visit_i32(v)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_i64()");
        let v = self.parse_integer()?;
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_u8() top = {}", self.state.get_top());
        let v = self.parse_integer()?;
        visitor.visit_u8(v)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_u16()");
        let v = self.parse_integer()?;
        visitor.visit_u16(v)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_u32()");
        let v = self.parse_integer()?;
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_u64()");
        let v = self.parse_integer()?;
        visitor.visit_u64(v)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_f32()");
        let v = self.parse_float()?;
        visitor.visit_f32(v)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_f64()");
        let v = self.parse_float()?;
        visitor.visit_f64(v)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_char()");
        let s = self
            .parse_str()
            .ok_or_else(|| {
                error!("unable to deserialize as char");
                Error::new("unable to deserialize as char")
            })?;
        let chars: Vec<char> = s.chars().collect();
        if chars.len() == 1 {
            visitor.visit_char(chars[0])
        } else {
            error!("unable to deserialize as char");
            Err(Error::new("unable to deserialize as char"))
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_str()");
        let s = self
            .parse_str()
            .ok_or_else(|| {
                error!("unable to deserialize as str");
                Error::new("unable to deserialize as str")
            })?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_string()");
        let s = self
            .parse_str()
            .ok_or_else(|| {
                error!("unable to deserialize as string");
                Error::new("unable to deserialize as string")
            })?;
        visitor.visit_string(s.to_string())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_bytes()");
        let v = self
            .parse_bytes()
            .ok_or_else(|| {
                error!("unable to deserialize as bytes");
                Error::new("unable to deserialize as bytes")
            })?;
        visitor.visit_borrowed_bytes(v)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_byte_buf()");
        let v = self
            .parse_bytes()
            .ok_or_else(|| {
                error!("unable to deserialize as bytes");
                Error::new("unable to deserialize as bytes")
            })?;
        visitor.visit_byte_buf(v.to_vec())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_option()");
        match self.state.value_type(-1) {
            state::LUA_TNIL => self.deserialize_unit(visitor),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_unit()");
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_unit_struct() name = {}", name);
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_newtype_struct() name = {}", name);
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_seq()");

        unsafe {
            let state = self.state.as_ptr();

            // push the length of the table
            ffi::lua_len(state, -1);

            // get the length from the stack
            let len = ffi::lua_tointeger(state, -1);

            // pop one element (the length) from the stack
            pop(state, 1);

            // create a reference to the table into the Lua registry
            let tref = LRef::register(&self.state);

            // give the visitor access to each element of the sequence
            let value = visitor.visit_seq(SeqAccessor::new(&mut self, &tref, len))?;

            // get the table from the registry and push it onto the stack
            tref.get();

            Ok(value)
        }
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_tuple() len = {}", len);
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_tuple_struct() name = {}, len = {}", name, len);
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_map()");

        unsafe {
            let state = self.state.as_ptr();

            // create a reference into the Lua registry to the table
            let tref = LRef::register(&self.state);

            // push the first key onto the stack
            ffi::lua_pushnil(state);

            // create a reference into the Lua registry to this key
            let kref = LRef::register(&self.state);

            // create an empty reference into the Lua registry for the value 
            let vref = LRef::empty(&self.state, ffi::LUA_REGISTRYINDEX);

            // give the visitor access to each element of the sequence
            let value = visitor.visit_map(MapAccessor::new(&mut self, &tref, &kref, &vref))?;

            // get the table from the registry and push it onto the stack
            tref.get();

            Ok(value)
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_struct() name = {}, fields = {:?}", name, fields);
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_enum() name = {}, variants = {:?}", name, variants);
        
        unsafe {
            let state = self.state.as_ptr();

            match self.state.value_type(-1) {
                ffi::LUA_TSTRING => {
                    // visit a unit variant
                    let s = self
                        .parse_str()
                        .ok_or_else(|| {
                            error!("non-string variant in enum");
                            Error::new("non-string variant in enum")
                        })?;
                    visitor.visit_enum(s.to_string().into_deserializer())
                }
                ffi::LUA_TTABLE => {
                    // store the table in the registry
                    let tref = ffi::luaL_ref(state, ffi::LUA_REGISTRYINDEX);

                    // visit a newtype variant, tuple variant, or struct variant
                    let ret = visitor.visit_enum(EnumAccessor::new(self, tref));

                    // get the table from the registry and push it onto the stack
                    ffi::lua_rawgeti(state, ffi::LUA_REGISTRYINDEX, tref.into());

                    // release the reference to the table
                    ffi::luaL_unref(state, ffi::LUA_REGISTRYINDEX, tref);

                    ret
                }
                _ => {
                    error!("unsupported enum value");
                    return Err(Error::new("unsupported enum value"))
                }
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_identifier() -> deserialize_str()");
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("deserialize_ignored_any() -> deserialize_any()");
        self.deserialize_any(visitor)
    }
}

struct MapAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    tref: &'a LRef,
    kref: &'a LRef,
    vref: &'a LRef,
}

impl<'a, 'de> MapAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, tref: &'a LRef, kref: &'a LRef, vref: &'a LRef) -> Self {
        Self {
            de,
            tref,
            kref,
            vref,
        }
    }
}

impl<'a, 'de> MapAccess<'de> for MapAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        trace!("MapAccessor::next_key_seed() top = {}", self.de.state.get_top());

        /// Returns `true` when there is a serializable element in the table on the top of the stack.
        ///
        /// # Safety
        ///
        /// Unsafe as this function leaves an additional element onto the stack.
        unsafe fn push_next(state: *mut ffi::lua_State, index: i32) -> bool {
            trace!("MapAccessor::next_element_seed::push_next() index = {}", index);

            // pops a key from the stack, and pushes a key-value pair from the table at the given
            // index
            while ffi::lua_next(state, index) != 0 {
                // check if the key (-2) and value (-1) are serializable
                if is_serializable(state, -1) && is_serializable(state, -2) {
                    // we found a serializable element, return!
                    return true;
                }

                // item is not serializable, ignore it; this means we pop it
                // from the stack
                pop(state, 1);
            }

            false
        }

        unsafe {
            let state = self.de.state.as_ptr();

            // get the table from the registry and push it onto the stack
            self.tref.get();

            // take the key out of the registry and push it onto the stack
            self.kref.take();

            // check if there is is still a serializable element in the sequence
            // and, if so, push it onto the stack
            if push_next(state, -2) {
                // replace the value reference with the value on top of the stack
                self.vref.replace();

                // deserialize the key
                let ret = seed.deserialize(&mut *self.de).map(Some);

                // replace the key reference with the value on top of the stack
                self.kref.replace();

                // pop the table from the stack
                pop(state, 1);

                ret
            } else {
                // pop the table from the stack
                pop(state, 1);

                Ok(None)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        trace!("MapAccessor::next_value_seed() top = {}", self.de.state.get_top());

        unsafe {
            let state = self.de.state.as_ptr();

            // take the value from the registry and push onto the stack
            self.vref.take();

            // deserialie the value
            let ret = seed.deserialize(&mut *self.de);

            // pop the value from the stack
            pop(state, 1);

            ret
        }
    }
}

struct SeqAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    tref: &'a LRef,
    len: i64,
    n: i64,
}

impl<'a, 'de> SeqAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, tref: &'a LRef, len: i64) -> Self {
        Self {
            de,
            tref,
            len,
            n: 1,
        }
    }
}

impl<'a, 'de> SeqAccess<'de> for SeqAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        trace!("SeqAccessor::next_element_seed() n = {}", self.n);

        // check if there are more elements expected
        if self.n > self.len {
            return Ok(None);
        }

        /// Returns `true` when there is a serializable element in the table on the top of the stack.
        ///
        /// # Safety
        ///
        /// Unsafe as this function leaves an additional element onto the stack.
        unsafe fn push_next(
            state: *mut ffi::lua_State,
            i: &mut i64,
            len: i64,
        ) -> bool {
            trace!("SeqAccessor::next_element_seed::push_next() i = {}, len = {}", i, len);

            while *i <= len {
                // push the next array element onto the stack
                ffi::lua_geti(state, -1, *i);
                *i += 1;

                // check if this item is serializable element
                if is_serializable(state, -1) {
                    // we found a serializable element, return!
                    return true;
                }

                // item is not serializable, ignore it; this means we pop it
                // from the stack
                pop(state, 1);
            }

            false
        }

        unsafe {
            let state = self.de.state.as_ptr();

            // get the table from the registry and push it onto the stack
            self.tref.get();
            
            // check if there is is still a serializable element in the sequence
            // and, if so, push it onto the stack
            if push_next(state, &mut self.n, self.len) {
                // deserialize the array element
                let ret = seed.deserialize(&mut *self.de).map(Some);

                // pop the table and array element from the stack
                pop(state, 2);

                ret
            } else {
                // pop the table from the stack
                pop(state, 1);

                Ok(None)
            }
        }
    }
}

struct EnumAccessor<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    tref: i32,
}

impl<'a, 'de> EnumAccessor<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, tref: i32) -> Self {
        Self { de, tref }
    }
}

impl<'a, 'de> EnumAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        trace!("EnumAccessor::variant_seed() top = {}", self.de.state.get_top());

        unsafe {
            let state = self.de.state.as_ptr();

            // get the table from the registry and push it onto the stack
            ffi::lua_rawgeti(state, ffi::LUA_REGISTRYINDEX, self.tref.into());

            // The `deserialize_enum` method parsed a `table` character so we are
            // currently inside of a map. The seed will be deserializing itself from
            // the key of the map.
            let ret = seed.deserialize(&mut *self.de);

            // pop the table from the stack
            pop(state, 1);

            ret.map(|value| (value, self))
        }
    }
}

impl<'a, 'de> VariantAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<(), Self::Error> {
        trace!("EnumAccessor::unit_variant() top = {}", self.de.state.get_top());
        error!("EnumAccessor::unit_variant() expected string");
        Err(Error::new("expected string"))
    }

    // Newtype variants are represented in Lua as `{ name = value }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        trace!("EnumAccessor::newtype_variant_seed() top = {}", self.de.state.get_top());
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in Lua as `{ value = {data...} }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("EnumAccessor::tuple_variant() top = {}", self.de.state.get_top());
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in Lua as `{ name = { k = v, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        trace!("EnumAccessor::struct_variant() top = {}", self.de.state.get_top());
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}

pub struct Error {
    msg: String,
}

impl Error {
    fn new<T: fmt::Display>(msg: T) -> Self {
        Error {
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.msg, f)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::new(msg)
    }
}

/// Returns `true` when the value at given `index` is serializable.
unsafe fn is_serializable(state: *mut ffi::lua_State, index: i32) -> bool {
    match ffi::lua_type(state, index) {
        ffi::LUA_TTABLE
        | ffi::LUA_TNIL
        | ffi::LUA_TSTRING
        | ffi::LUA_TNUMBER
        | ffi::LUA_TBOOLEAN => true,
        _ => false,
    }
}

unsafe fn pop(state: *mut ffi::lua_State, n: i32) {
    trace!("pop() top = {}, n = {}", ffi::lua_gettop(state), n);
    ffi::lua_pop(state, n)
}