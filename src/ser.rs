//! Lua serialization.
use std::{convert::TryFrom, fmt};

use serde::{ser, Serialize};

use crate::{ffi, State};

pub struct Error {
    msg: String,
}

pub struct TableSerializer<'a> {
    state: &'a mut State,
    tref: i32,
    i: i64,
}

pub struct TableVariantSerializer<'a> {
    state: &'a mut State,
    tref: i32,
    kref: i32,
    i: i64,
}

macro_rules! check_stack {
    ($state:expr, $n:expr) => {
        if $state.as_stack().reserve($n) {
            Ok(())
        } else {
            Err(Error { msg: "".into() })
        }
    };
}

impl<'a> ser::Serializer for &'a mut State {
    type Ok = i32;
    type Error = Error;

    type SerializeSeq = TableSerializer<'a>;
    type SerializeTuple = TableSerializer<'a>;
    type SerializeTupleStruct = TableSerializer<'a>;
    type SerializeTupleVariant = TableVariantSerializer<'a>;
    type SerializeMap = TableSerializer<'a>;
    type SerializeStruct = TableSerializer<'a>;
    type SerializeStructVariant = TableVariantSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 1 value
        check_stack!(self, 1)?;

        // push the boolean on the stack; C expects this as an i32, so convert
        // it first
        let b = if v { 1 } else { 0 };
        unsafe { ffi::lua_pushboolean(self.as_ptr(), b) };

        Ok(1)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 1 value
        check_stack!(self, 1)?;

        // push the signed integer onto the stack
        unsafe { ffi::lua_pushinteger(self.as_ptr(), v) };

        Ok(1)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v.into())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::try_from(v).map_err(|e| Error { msg: e.to_string() })?)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v.into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 1 value
        check_stack!(self, 1)?;

        // push the float onto the stack
        unsafe { ffi::lua_pushnumber(self.as_ptr(), v) };

        Ok(1)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 1 value
        check_stack!(self, 1)?;

        // push the byte slice onto the stack
        unsafe { ffi::lua_pushlstring(self.as_ptr(), v.as_ptr() as _, v.len()) };

        Ok(1)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 1 value
        check_stack!(self, 1)?;

        // push nil on to the stack
        unsafe { ffi::lua_pushnil(self.as_ptr()) };

        Ok(1)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for the table and the value
        check_stack!(self, 2)?;

        unsafe {
            // create a table and push it onto the stack
            ffi::lua_createtable(self.as_ptr(), 0, 1);

            // serialize the value and push it on the stack
            let i = value.serialize(&mut *self)?;
            debug_assert_eq!(1, i, "expected that a serialized value takes 1 stack slot");

            // push the value into the table with the variant as key
            ffi::lua_setfield(self.as_ptr(), -2, variant.as_ptr() as _);
        }

        Ok(1)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        // ensure stack has space for the table
        check_stack!(self, 1)?;

        // use the provided len as the hint for the number of array elements, if
        // None is provided use 0 as hint.
        let len = len.unwrap_or(0);
        let narr = i32::try_from(len).map_err(|e| Error { msg: e.to_string() })?;

        let tref = unsafe {
            // create a table and push it onto the stack
            ffi::lua_createtable(self.as_ptr(), narr, 0);

            // create a reference to the just created table, this pops the table
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        Ok(TableSerializer {
            state: self,
            tref,
            i: 1, // Lua arrays start at index 1.
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        // ensure stack has space for 2 x 1 temp record
        check_stack!(self, 1)?;

        // use the provided len as the hint for the number of array elements
        let narr = i32::try_from(len).map_err(|e| Error { msg: e.to_string() })?;

        let tref = unsafe {
            // create a table and push it onto the stack
            ffi::lua_createtable(self.as_ptr(), narr, 0);

            // create a reference to the just created table, this pops the table
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        let kref = unsafe {
            // push the variant on the stack
            ffi::lua_pushlstring(self.as_ptr(), variant.as_ptr() as _, variant.len());

            // create a reference to the just pushed string, this pops the string
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        Ok(TableVariantSerializer {
            state: self,
            tref,
            kref,
            i: 1, // Lua arrays start at index 1.
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        // ensure stack has space for the table
        check_stack!(self, 1)?;

        // use the provided len as the hint for the number of record elements, if
        // None is provided use 0 as hint.
        let len = len.unwrap_or(0);
        let nrec = i32::try_from(len).map_err(|e| Error { msg: e.to_string() })?;

        let tref = unsafe {
            // create a table and push it onto the stack
            ffi::lua_createtable(self.as_ptr(), 0, nrec);

            // create a reference to the just created table, this pops the table
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        Ok(TableSerializer {
            state: self,
            tref,
            i: ffi::LUA_REFNIL.into(), // we're using this to hold the key references
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        // ensure stack has space for 2 x 1 temp record
        check_stack!(self, 1)?;

        // use the provided len as the hint for the number of record elements.
        let nrec = i32::try_from(len).map_err(|e| Error { msg: e.to_string() })?;

        let tref = unsafe {
            // create a table and push it onto the stack
            ffi::lua_createtable(self.as_ptr(), 0, nrec);

            // create a reference to the just created table, this pops the table
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        let kref = unsafe {
            // push the variant on the stack
            ffi::lua_pushlstring(self.as_ptr(), variant.as_ptr() as _, variant.len());

            // create a reference to the just pushed string, this pops the string
            // from the stack
            ffi::luaL_ref(self.as_ptr(), ffi::LUA_REGISTRYINDEX)
        };

        Ok(TableVariantSerializer {
            state: self,
            tref,
            kref,
            i: 1, // Lua arrays start at index 1.
        })
    }
}

impl<'a> ser::SerializeSeq for TableSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for 1 element (the table; the serialization of
        // the value will check its required stack size)
        check_stack!(self.state, 1)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // serialie the value, pushing it onto the stack
            let n = value.serialize(&mut *self.state)?;
            debug_assert_eq!(n, 1, "expected value to be serialized in one slot");

            // append the value to the table, this pops the value from the stack
            ffi::lua_seti(self.state.as_ptr(), -2, self.i);
            self.i += 1;

            // pop the table from the stack
            ffi::lua_pop(self.state.as_ptr(), 1);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for the table
        check_stack!(self.state, 1)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // release the table reference in the Lua registry
            ffi::luaL_unref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref);
        }

        Ok(1)
    }
}

impl<'a> ser::SerializeTuple for TableSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for TableSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for TableVariantSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for 1 element (the table; the serialization of
        // the value will check its required stack size)
        check_stack!(self.state, 1)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // serialie the value, pushing it onto the stack
            let n = value.serialize(&mut *self.state)?;
            debug_assert_eq!(n, 1, "expected value to be serialized in one slot");

            // append the value to the table, this pops the value from the stack
            ffi::lua_seti(self.state.as_ptr(), -2, self.i);
            self.i += 1;

            // pop the table from the stack
            ffi::lua_pop(self.state.as_ptr(), 1);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for 3 elements (the key, the inner table and
        // the outer table)
        check_stack!(self.state, 3)?;

        unsafe {
            // create the outer table and push it onto the stack
            ffi::lua_createtable(self.state.as_ptr(), 0, 1);

            // retrieve the key from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.kref.into());
            debug_assert_ne!(t, ffi::LUA_TNIL, "expected a key");

            // release the key reference in the Lua registry
            ffi::luaL_unref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.kref);

            // retrieve the inner table (that is the value of the outer table)
            // from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // append the value to the table, this pops both the key and the value 
            // from the stack
            ffi::lua_settable(self.state.as_ptr(), -3);

            // release the table reference in the Lua registry
            ffi::luaL_unref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref);
        }

        Ok(1)
    }
}

impl<'a> ser::SerializeMap for TableSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for 1 element (the table; the serialization of
        // the key will check its required stack size)
        check_stack!(self.state, 1)?;

        // serialie the key, pushing it onto the stack
        let n = key.serialize(&mut *self.state)?;
        debug_assert_eq!(n, 1, "expected value to be serialized in one slot");

        self.i = unsafe {
            // create a reference to the pushed key, this pops the key value
            // from the stack
            ffi::luaL_ref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX).into()
        };

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for 2 element (the table and the key; the 
        // serialization of the value will check its required stack size)
        check_stack!(self.state, 2)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // retrieve the key from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.i);
            debug_assert_ne!(t, ffi::LUA_TNIL, "expected a key");

            // release the key reference in the Lua registry
            //
            // SAFETY: The unwrap is ok as this i64 value was originally coerced
            // from an i32 value.
            ffi::luaL_unref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, i32::try_from(self.i).unwrap());

            // serialie the value, pushing it onto the stack
            let n = value.serialize(&mut *self.state)?;
            debug_assert_eq!(n, 1, "expected value to be serialized in one slot");

            // append the value to the table, this pops both the key and the value 
            // from the stack
            ffi::lua_settable(self.state.as_ptr(), -3);

            // pop the table from the stack
            ffi::lua_pop(self.state.as_ptr(), 1);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // ensure stack has space for the table
        check_stack!(self.state, 1)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // release the table reference in the Lua registry
            ffi::luaL_unref(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref);
        }

        Ok(1)
    }
}

impl<'a> ser::SerializeStruct for TableSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_key(self, key)?;
        ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeMap::end(self)
    }
}

impl<'a> ser::SerializeStructVariant for TableVariantSerializer<'a> {
    type Ok = i32;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // ensure stack has space for 1 element (the table; the serialization of
        // the key and value will check its required stack size)
        check_stack!(self.state, 1)?;

        unsafe {
            // retrieve the table from the Lua registry, pushing it onto the stack
            let t = ffi::lua_rawgeti(self.state.as_ptr(), ffi::LUA_REGISTRYINDEX, self.tref.into());
            debug_assert_eq!(t, ffi::LUA_TTABLE, "expected a table");

            // serialie the key, pushing it onto the stack
            let n = key.serialize(&mut *self.state)?;
            debug_assert_eq!(n, 1, "expected key to be serialized in one slot");

            // serialie the value, pushing it onto the stack
            let n = value.serialize(&mut *self.state)?;
            debug_assert_eq!(n, 1, "expected value to be serialized in one slot");

            // append the value to the table, this pops both the key and the value 
            // from the stack
            ffi::lua_settable(self.state.as_ptr(), -3);

            // pop the table from the stack
            ffi::lua_pop(self.state.as_ptr(), 1);
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeTupleVariant::end(self)
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

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }
}
